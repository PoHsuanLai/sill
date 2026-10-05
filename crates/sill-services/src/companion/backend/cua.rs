//! The computer-use runs cuad has, as `RunFacts`: the manager's list and its `RunAdded` and
//! `RunRemoved`, and one follower per run over its properties and signals. Controls are the run
//! object's own methods.

use std::collections::HashMap;

use cua_bus::{BudgetBody, CuaOutcome, RunEnd, RunStateSlug, WindowBody};
use cua_dbus::{CUA_BUS, ManagerProxy, RunProxy};
use docket_core::RunMode;
use prov::RunId;
use sill_launcher::RunControl;
use sill_model::companion::{CompanionEvent, RunFacts};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::text::{from_json, text};
use crate::owner;
use crate::stream::next;

/// The run an object path names: cuad mints `r-<n>` and an object path cannot hold the dash,
/// so `r_<n>` is `r-<n>`; any other segment is the id as written.
pub(super) fn run_id_of(path: &str) -> Option<RunId> {
    let segment = path.rsplit('/').next()?;
    let id = match segment.strip_prefix("r_") {
        Some(number) if number.chars().all(|c| c.is_ascii_digit()) => format!("r-{number}"),
        _ => segment.to_owned(),
    };
    RunId::parse(&id).ok()
}

/// Follow cuad while it is on the bus: every run it has, as it adds and removes them.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    loop {
        owner::appeared(&connection, CUA_BUS).await;
        tokio::select! {
            () = manage(&connection, &events) => return,
            () = owner::changed(&connection, CUA_BUS) => {}
        }
    }
}

/// The manager's runs now and as they change; ends when the events have no receiver.
async fn manage(connection: &Connection, events: &UnboundedSender<CompanionEvent>) {
    let Ok(manager) = ManagerProxy::new(connection).await else {
        return std::future::pending().await;
    };
    let (Ok(mut added), Ok(mut removed)) = (
        manager.receive_run_added().await,
        manager.receive_run_removed().await,
    ) else {
        return std::future::pending().await;
    };
    let mut followers: HashMap<String, JoinHandle<()>> = HashMap::new();
    for path in manager.runs().await.unwrap_or_default() {
        watch(connection, &mut followers, path, events);
    }
    loop {
        tokio::select! {
            Some(signal) = next(&mut added) => {
                if let Ok(args) = signal.args() {
                    watch(connection, &mut followers, args.run().clone(), events);
                }
            }
            Some(signal) = next(&mut removed) => {
                if let Ok(args) = signal.args() {
                    let path = args.run().to_string();
                    if let Some(follower) = followers.remove(&path) {
                        follower.abort();
                    }
                    if let Some(run) = run_id_of(&path)
                        && events.send(CompanionEvent::RunGone(run)).is_err()
                    {
                        return;
                    }
                }
            }
            else => return,
        }
    }
}

/// Start following `path` unless it already is.
fn watch(
    connection: &Connection,
    followers: &mut HashMap<String, JoinHandle<()>>,
    path: OwnedObjectPath,
    events: &UnboundedSender<CompanionEvent>,
) {
    let key = path.to_string();
    if followers.contains_key(&key) {
        return;
    }
    let (connection, events) = (connection.clone(), events.clone());
    let handle = tokio::spawn(async move {
        if let Err(why) = follow_run(connection, path, events).await {
            eprintln!("sill: companion: following a run: {why}");
        }
    });
    followers.insert(key, handle);
}

/// One run: its facts now, then again after each thing it says.
async fn follow_run(
    connection: Connection,
    path: OwnedObjectPath,
    events: UnboundedSender<CompanionEvent>,
) -> Result<(), String> {
    let run = run_id_of(path.as_str()).ok_or_else(|| format!("{path} names no run"))?;
    let proxy = RunProxy::builder(&connection)
        .path(path)
        .map_err(text)?
        .build()
        .await
        .map_err(text)?;
    let mut phases = proxy.receive_phase_changed().await.map_err(text)?;
    let mut steps = proxy.receive_step_started().await.map_err(text)?;
    let mut thoughts = proxy.receive_thought_delta().await.map_err(text)?;
    let mut ends = proxy.receive_finished().await.map_err(text)?;
    let mut facts = read(&run, &proxy).await?;
    let send = |facts: &RunFacts| events.send(CompanionEvent::Run(Box::new(facts.clone())));
    if send(&facts).is_err() {
        return Ok(());
    }
    loop {
        tokio::select! {
            Some(signal) = next(&mut phases) => {
                if let Some(slug) = signal.args().ok().and_then(|a| slug_of(a.state())) {
                    facts = RunFacts { state: slug, ..facts };
                }
            }
            Some(signal) = next(&mut steps) => {
                if let Ok(args) = signal.args() {
                    facts = RunFacts { step: *args.step(), thought: None, ..facts };
                }
            }
            Some(signal) = next(&mut thoughts) => {
                if let Ok(args) = signal.args() {
                    let said = format!("{}{}", facts.thought.as_deref().unwrap_or_default(), args.text());
                    facts = RunFacts { thought: Some(said), ..facts };
                }
            }
            Some(signal) = next(&mut ends) => {
                if let Some(end) = signal.args().ok().and_then(|a| from_json::<RunEnd>(a.end()).ok()) {
                    facts = RunFacts {
                        state: RunStateSlug::Finished,
                        outcome: Some(end.outcome),
                        ..facts
                    };
                }
            }
            else => return Ok(()),
        }
        if send(&facts).is_err() {
            return Ok(());
        }
    }
}

fn slug_of(text: &str) -> Option<RunStateSlug> {
    serde_json::from_value(serde_json::Value::String(text.to_owned())).ok()
}

/// The run's properties as facts.
async fn read(run: &RunId, proxy: &RunProxy<'_>) -> Result<RunFacts, String> {
    let state = slug_of(&proxy.state().await.map_err(text)?).ok_or("an unknown run state")?;
    let mode: RunMode =
        serde_json::from_value(serde_json::Value::String(proxy.mode().await.map_err(text)?))
            .map_err(text)?;
    let window = proxy
        .window()
        .await
        .ok()
        .and_then(|body| from_json::<WindowBody>(&body).ok())
        .map(|body| body.title.0);
    let budget = proxy
        .budget()
        .await
        .ok()
        .and_then(|body| from_json::<BudgetBody>(&body).ok());
    Ok(RunFacts {
        run: run.clone(),
        goal: proxy.goal().await.map_err(text)?,
        window,
        step: proxy.step().await.map_err(text)?,
        state,
        outcome: None::<CuaOutcome>,
        mode,
        thought: None,
        budget,
    })
}

/// Do `control` to `run`: the run object's own method.
pub(super) async fn control(
    connection: &Connection,
    run: &RunId,
    control: RunControl,
) -> Result<(), String> {
    let path = cua_dbus::run_path(run);
    let proxy = RunProxy::builder(connection)
        .path(path)
        .map_err(text)?
        .build()
        .await
        .map_err(text)?;
    match control {
        RunControl::Pause => proxy.pause().await.map_err(text),
        RunControl::Resume => proxy.resume().await.map_err(text),
        RunControl::Stop => proxy.cancel().await.map_err(text),
        RunControl::TakeOver => proxy.take_over().await.map_err(text),
        RunControl::HandBack => proxy.hand_back().await.map_err(text),
        // Watching is the compositor's: it shows the run's window; and undoing is intentd's,
        // which the caller sends instead.
        RunControl::Watch | RunControl::UndoAll => Ok(()),
    }
}
