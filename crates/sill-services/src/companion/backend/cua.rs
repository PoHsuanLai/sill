//! The computer-use runs cuad has, as `RunFacts`: the manager's list and its `RunAdded` and
//! `RunRemoved`, and one follower per run over its properties and signals. A run is named by its
//! `Id` property, read once when its object appears; the path is only where to find it, so each
//! path is remembered with its id for `RunRemoved`, which comes after the object is gone.
//! Controls are the run object's own methods.

use std::collections::HashMap;

use cua_bus::{BudgetBody, CuaOutcome, RunEnd, RunStateSlug, WindowBody};
use cua_dbus::{CUA_BUS, ManagerProxy, RunProxy};
use docket_core::RunMode;
use prov::RunId;
use sill_launcher::RunControl;
use sill_model::companion::{CompanionEvent, RunFacts, SightState};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::text::from_json;
use crate::call_error::CallError;
use crate::owner;
use crate::stream::next;

/// Follow cuad while it is on the bus: every run it has, as it adds and removes them.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    owner::while_owned(
        &connection,
        CUA_BUS,
        async || manage(&connection, &events).await,
        // With cuad gone nothing is known about the screen model, so nothing is said. A send
        // that fails means the events have no receiver, which ends `manage` on its own.
        || {
            let _ = events.send(CompanionEvent::Sight(SightState::Sees));
        },
    )
    .await;
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
    // Older cuad has no `SightNote`: it then says nothing.
    let mut notes = manager.receive_sight_note_changed().await;
    let sight = |note: Option<String>| {
        events.send(CompanionEvent::Sight(SightState::of(
            &note.unwrap_or_default(),
        )))
    };
    if sight(manager.sight_note().await.ok()).is_err() {
        return;
    }
    let mut followers: HashMap<String, Follower> = HashMap::new();
    for path in manager.runs().await.unwrap_or_default() {
        watch(connection, &mut followers, path, events).await;
    }
    loop {
        tokio::select! {
            Some(signal) = next(&mut added) => {
                if let Ok(args) = signal.args() {
                    watch(connection, &mut followers, args.run().clone(), events).await;
                }
            }
            Some(signal) = next(&mut removed) => {
                if let Ok(args) = signal.args() {
                    let path = args.run().to_string();
                    if let Some(Follower { run, task }) = followers.remove(&path) {
                        task.abort();
                        if events.send(CompanionEvent::RunGone(run)).is_err() {
                            return;
                        }
                    }
                }
            }
            Some(changed) = next(&mut notes) => {
                if sight(changed.get().await.ok()).is_err() {
                    return;
                }
            }
            else => return,
        }
    }
}

/// A run being followed: the id its object named itself by, and the task following it.
struct Follower {
    run: RunId,
    task: JoinHandle<()>,
}

/// Start following `path` unless it already is: read its `Id`, then follow its facts. A run
/// that names no id cannot be shown, and says why.
async fn watch(
    connection: &Connection,
    followers: &mut HashMap<String, Follower>,
    path: OwnedObjectPath,
    events: &UnboundedSender<CompanionEvent>,
) {
    let key = path.to_string();
    if followers.contains_key(&key) {
        return;
    }
    let proxy = match run_proxy(connection, path).await {
        Ok(proxy) => proxy,
        Err(why) => return eprintln!("sill: companion: following a run: {why}"),
    };
    let run = match proxy
        .id()
        .await
        .map_err(CallError::from)
        .and_then(|id| RunId::parse(&id).map_err(CallError::invalid))
    {
        Ok(run) => run,
        Err(why) => return eprintln!("sill: companion: a run's id: {why}"),
    };
    let (events, named) = (events.clone(), run.clone());
    let task = tokio::spawn(async move {
        if let Err(why) = follow_run(named, proxy, events).await {
            eprintln!("sill: companion: following a run: {why}");
        }
    });
    followers.insert(key, Follower { run, task });
}

async fn run_proxy(
    connection: &Connection,
    path: OwnedObjectPath,
) -> Result<RunProxy<'static>, CallError> {
    RunProxy::builder(connection)
        .path(path)?
        .build()
        .await
        .map_err(CallError::from)
}

/// One run: its facts now, then again after each thing it says.
async fn follow_run(
    run: RunId,
    proxy: RunProxy<'static>,
    events: UnboundedSender<CompanionEvent>,
) -> Result<(), CallError> {
    let mut phases = proxy.receive_phase_changed().await?;
    let mut steps = proxy.receive_step_started().await?;
    let mut thoughts = proxy.receive_thought_delta().await?;
    let mut ends = proxy.receive_finished().await?;
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
async fn read(run: &RunId, proxy: &RunProxy<'_>) -> Result<RunFacts, CallError> {
    let state =
        slug_of(&proxy.state().await?).ok_or_else(|| CallError::invalid("an unknown run state"))?;
    let mode: RunMode = serde_json::from_value(serde_json::Value::String(proxy.mode().await?))?;
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
        goal: proxy.goal().await?,
        window,
        step: proxy.step().await?,
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
) -> Result<(), CallError> {
    let path = cua_dbus::run_path(run);
    let proxy = RunProxy::builder(connection).path(path)?.build().await?;
    match control {
        RunControl::Pause => proxy.pause().await.map_err(CallError::from),
        RunControl::Resume => proxy.resume().await.map_err(CallError::from),
        RunControl::Stop => proxy.cancel().await.map_err(CallError::from),
        RunControl::TakeOver => proxy.take_over().await.map_err(CallError::from),
        RunControl::HandBack => proxy.hand_back().await.map_err(CallError::from),
        // Watching is the compositor's: it shows the run's window; and undoing is intentd's,
        // which the caller sends instead.
        RunControl::Watch | RunControl::UndoAll => Ok(()),
    }
}
