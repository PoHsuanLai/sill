//! Asking the companion and following what it answers: `Companion1.Open` for a session,
//! `Intents1.Session.Turn` for the person's turn (the router records it, as role launcher),
//! `Companion1.Ask` with the turn and what the person kept, then the answer object's `View` and
//! `Updated`. A card button is `Answer.Act`, which answers at once: what the call comes to is the
//! answer's own plan, which `Updated` carries.

use companion_wire::{AnswerWire, AskWire, RouteNote};
use docket_client::{DbusTransport, Intents};
use docket_core::{TurnIn, TurnSource, UserTurn};
use docket_dbus::{CompanionAnswerProxy, CompanionProxy, Details, answer_path};
use docket_permissions_core::Mode;
use docket_vocab::{CardActionId, Origin, WindowKey};
use docket_wire::{SessionOpen, SessionOpened};
use prov::{AgentRef, SessionId, TaskId, UnixSeconds};
use sill_launcher::WindowKey as ShellWindow;
use sill_model::companion::{AskInput, CompanionEvent, ModeMsg};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::text::{from_json, to_json};
use crate::call_error::CallError;
use crate::deadline::{COMPANION_CALL, within};
use crate::stream::next;

/// The shell's window as the companion names it: `w` and the compositor's number.
fn window_key(window: Option<ShellWindow>) -> Result<WindowKey, CallError> {
    let name = format!("w{}", window.map_or(0, |window| window.0));
    WindowKey::parse(&name).map_err(CallError::invalid)
}

/// Send `input` to the companion: open a session, record the turn, ask. Returns the answer's
/// object, which the caller follows. The turn carries `mode` (the conversation's, when the router
/// has told one) so the router puts the new session in it before the turn is recorded.
pub(super) async fn ask(
    connection: &Connection,
    input: AskInput,
    mode: Option<Mode>,
    at: UnixSeconds,
    events: &UnboundedSender<CompanionEvent>,
) -> Result<OwnedObjectPath, CallError> {
    let companion = CompanionProxy::new(connection).await?;
    let open = SessionOpen {
        space: input.space,
        agent: AgentRef::Companion,
        parent: None,
        cwd: None,
        started_from: None,
        external: None,
        helper: None,
        acts_for: None,
    };
    let opened: SessionOpened = from_json(&companion.open(&to_json(&open)?).await?)?;
    let _ = events.send(CompanionEvent::Mode(ModeMsg::Opened {
        session: opened.session.clone(),
        mode: opened.mode,
    }));
    let intents = Intents::over(DbusTransport::new(connection.clone()));
    let turn = TurnIn {
        text: input.prompt.clone(),
        origin: Origin::Launcher,
        keep: input.keep,
        via: input.via,
        mode,
    };
    let id = intents.session_turn(opened.session.clone(), turn).await?;
    let recorded = UserTurn {
        id,
        text: input.prompt.clone(),
        at,
        from: TurnSource::Launcher,
        via: input.via,
        mode,
    };
    let session = opened.session;
    let wire = AskWire {
        session: session.clone(),
        turn: recorded,
        keep: input.keep,
        parent_window: window_key(input.window)?,
        app: input.app,
    };
    let path = companion.ask(&to_json(&wire)?, &Details::new()).await?;
    let _ = events.send(CompanionEvent::Asked {
        task: opened.task,
        prompt: input.prompt,
        session,
    });
    Ok(path)
}

/// Follow one answer object: its view now, then every `Updated`. Ends when the signal stream
/// does (the connection closed); the caller aborts it when the object goes away.
pub(super) async fn follow(
    connection: Connection,
    path: OwnedObjectPath,
    events: UnboundedSender<CompanionEvent>,
) {
    let Ok(builder) = CompanionAnswerProxy::builder(&connection).path(path) else {
        return;
    };
    let Ok(proxy) = builder.build().await else {
        return;
    };
    let Ok(mut updates) = proxy.receive_updated().await else {
        return;
    };
    // `Routing` changes while stages happen; an older companion has no such property.
    let mut routing = proxy.receive_routing_changed().await;
    // The session is read once, as the answer appears, and kept with its state.
    let session = proxy
        .session()
        .await
        .ok()
        .and_then(|session| SessionId::parse(&session).ok());
    let first = proxy
        .view()
        .await
        .ok()
        .and_then(|view| from_json::<AnswerWire>(&view).ok());
    let task = first.as_ref().map(|wire| wire.task.clone());
    if let Some(wire) = first
        && !publish(&events, wire, session.clone())
    {
        return;
    }
    if let Some(task) = &task {
        notes(&events, task, proxy.routing().await.ok());
    }
    loop {
        tokio::select! {
            Some(signal) = next(&mut updates) => {
                let Ok(args) = signal.args() else { continue };
                if let Ok(wire) = from_json::<AnswerWire>(args.view())
                    && !publish(&events, wire, session.clone())
                {
                    return;
                }
            }
            Some(changed) = next(&mut routing) => {
                if let Some(task) = &task {
                    notes(&events, task, changed.get().await.ok());
                }
            }
            else => return,
        }
    }
}

/// Tell the service what the answer's `Routing` says. A property that cannot be read or is not
/// notes (an older companion has none) says nothing.
fn notes(events: &UnboundedSender<CompanionEvent>, task: &TaskId, routing: Option<String>) {
    if let Some(notes) = routing.and_then(|json| from_json::<Vec<RouteNote>>(&json).ok()) {
        let _ = events.send(CompanionEvent::Routing {
            task: task.clone(),
            notes,
        });
    }
}

/// Tell the service the answer and the session its object names; `false` when it is gone.
fn publish(
    events: &UnboundedSender<CompanionEvent>,
    wire: AnswerWire,
    session: Option<SessionId>,
) -> bool {
    let task = wire.task.clone();
    events.send(CompanionEvent::Answer(Box::new(wire))).is_ok()
        && events
            .send(CompanionEvent::Session { task, session })
            .is_ok()
}

/// Press `action` on `task`'s card. The call answers at once; the press is over when the
/// answer's plan says its step is, so nothing waits here. A press the companion refuses is told.
pub(super) async fn card(
    connection: Connection,
    task: TaskId,
    action: CardActionId,
    events: UnboundedSender<CompanionEvent>,
) {
    let pressed = match within(
        COMPANION_CALL,
        "the assistant",
        press(&connection, &task, &action),
    )
    .await
    {
        Ok(done) => done,
        Err(silent) => Err(silent.into()),
    };
    if let Err(why) = pressed {
        eprintln!("sill: companion: a card button: {why}");
        let _ = events.send(CompanionEvent::CardRefused { task, action });
    }
}

async fn press(
    connection: &Connection,
    task: &TaskId,
    action: &CardActionId,
) -> Result<(), CallError> {
    let path = OwnedObjectPath::try_from(answer_path(task)).map_err(CallError::invalid)?;
    let proxy = CompanionAnswerProxy::builder(connection)
        .path(path)?
        .build()
        .await?;
    proxy
        .act(&to_json(action)?)
        .await
        .map(|_| ())
        .map_err(CallError::from)
}

/// Stop `task`.
pub(super) async fn cancel(connection: &Connection, task: &TaskId) -> Result<(), CallError> {
    let path = OwnedObjectPath::try_from(answer_path(task)).map_err(CallError::invalid)?;
    let proxy = CompanionAnswerProxy::builder(connection)
        .path(path)?
        .build()
        .await?;
    proxy.cancel().await.map_err(CallError::from)
}

/// Warm the planner: a prompt is about to be typed.
pub(super) async fn prepare(connection: &Connection) -> Result<(), CallError> {
    let proxy = CompanionProxy::new(connection).await?;
    proxy
        .prepare(&Details::new())
        .await
        .map_err(CallError::from)
}
