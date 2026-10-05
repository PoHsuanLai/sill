//! Asking the companion and following what it answers: `Companion1.Open` for a session,
//! `Intents1.Session.Turn` for the person's turn (the router records it, as role launcher),
//! `Companion1.Ask` with the turn and what the person kept, then the answer object's `View` and
//! `Updated`. A card button is `Answer.Act`, whose request ends with a `Response`.

use companion_wire::{AnswerWire, AskWire};
use docket_client::{DbusTransport, Intents, requested};
use docket_core::{
    CardActionId, Origin, SessionOpen, SessionOpened, TurnIn, TurnSource, TurnVia, UserTurn,
    WindowKey,
};
use docket_dbus::{COMPANION_BUS, CompanionAnswerProxy, CompanionProxy, Details, answer_path};
use prov::{AgentRef, SpaceId, TaskId, UnixSeconds};
use sill_launcher::WindowKey as ShellWindow;
use sill_model::companion::{AskInput, CompanionEvent};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::text::{from_json, text, to_json};
use crate::stream::next;

/// The shell's window as the companion names it: `w` and the compositor's number.
fn window_key(window: Option<ShellWindow>) -> Result<WindowKey, String> {
    let name = format!("w{}", window.map_or(0, |window| window.0));
    WindowKey::parse(&name).map_err(text)
}

/// Send `input` to the companion: open a session, record the turn, ask. Returns the answer's
/// object, which the caller follows.
pub(super) async fn ask(
    connection: &Connection,
    input: AskInput,
    at: UnixSeconds,
    events: &UnboundedSender<CompanionEvent>,
) -> Result<OwnedObjectPath, String> {
    let companion = CompanionProxy::new(connection).await.map_err(text)?;
    let open = SessionOpen {
        space: SpaceId::desktop(),
        agent: AgentRef::Companion,
        parent: None,
    };
    let opened: SessionOpened = from_json(&companion.open(&to_json(&open)?).await.map_err(text)?)?;
    let intents = Intents::over(DbusTransport::new(connection.clone()));
    let turn = TurnIn {
        text: input.prompt.clone(),
        origin: Origin::Launcher,
        keep: input.keep,
        via: TurnVia::Typed,
    };
    let id = intents
        .session_turn(opened.session.clone(), turn)
        .await
        .map_err(text)?;
    let recorded = UserTurn {
        id,
        text: input.prompt.clone(),
        at,
        from: TurnSource::Launcher,
        via: TurnVia::Typed,
    };
    let wire = AskWire {
        session: opened.session,
        turn: recorded,
        keep: input.keep,
        parent_window: window_key(input.window)?,
        app: input.app,
    };
    let path = companion
        .ask(&to_json(&wire)?, &Details::new())
        .await
        .map_err(text)?;
    let _ = events.send(CompanionEvent::Asked {
        task: opened.task,
        prompt: input.prompt,
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
    if let Some(wire) = proxy
        .view()
        .await
        .ok()
        .and_then(|view| from_json::<AnswerWire>(&view).ok())
        && events.send(CompanionEvent::Answer(wire)).is_err()
    {
        return;
    }
    while let Some(signal) = next(&mut updates).await {
        let Ok(args) = signal.args() else { continue };
        if let Ok(wire) = from_json::<AnswerWire>(args.view())
            && events.send(CompanionEvent::Answer(wire)).is_err()
        {
            return;
        }
    }
}

/// Press `action` on `task`'s card: the button is busy until its request has its `Response`.
pub(super) async fn card(
    connection: Connection,
    task: TaskId,
    action: CardActionId,
    events: UnboundedSender<CompanionEvent>,
) {
    let _ = events.send(CompanionEvent::CardPressed {
        task: task.clone(),
        action: action.clone(),
    });
    let ended = press(&connection, &task, &action).await;
    if let Err(why) = ended {
        eprintln!("sill: companion: a card button: {why}");
    }
    let _ = events.send(CompanionEvent::CardEnded(task));
}

async fn press(
    connection: &Connection,
    task: &TaskId,
    action: &CardActionId,
) -> Result<(), String> {
    let path = OwnedObjectPath::try_from(answer_path(task)).map_err(text)?;
    let proxy = CompanionAnswerProxy::builder(connection)
        .path(path)
        .map_err(text)?
        .build()
        .await
        .map_err(text)?;
    let body = to_json(action)?;
    let waited = requested(connection, COMPANION_BUS, proxy.act(&body))
        .await
        .map_err(text)?;
    waited.map(|_| ()).map_err(text)
}

/// Stop `task`.
pub(super) async fn cancel(connection: &Connection, task: &TaskId) -> Result<(), String> {
    let path = OwnedObjectPath::try_from(answer_path(task)).map_err(text)?;
    let proxy = CompanionAnswerProxy::builder(connection)
        .path(path)
        .map_err(text)?
        .build()
        .await
        .map_err(text)?;
    proxy.cancel().await.map_err(text)
}

/// Warm the planner: a prompt is about to be typed.
pub(super) async fn prepare(connection: &Connection) -> Result<(), String> {
    let proxy = CompanionProxy::new(connection).await.map_err(text)?;
    proxy.prepare(&Details::new()).await.map_err(text)
}
