//! What the shell asks intentd as the launcher: the things apps index, the undo journal, undoing,
//! and performing an action the person picked.

use std::collections::BTreeSet;

use docket_client::{ClientError, DbusTransport, Intents};
use docket_core::{
    CallRequest, Generation as SearchGeneration, Handle, Hit, JournalFilter, Mode, SearchAsk,
    SearchScope, UndoEntry, UndoScope, WireRefusal,
};
use docket_dbus::{RegistryProxy, SearchProxy, SessionProxy};
use porter_core::Count;
use prov::AppName;
use prov::SessionId;
use sill_launcher::Generation;
use sill_model::companion::{
    CompanionEvent, HelpersMsg, ModeMsg, Opener, offered_apps, openers_of,
};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use super::shown;
use super::text::from_json;
use crate::call_error::CallError;
use crate::stream::next;

fn intents(connection: &Connection) -> Intents<DbusTransport> {
    Intents::over(DbusTransport::new(connection.clone()))
}

/// How many journal rows the launcher reads.
const JOURNAL_ROWS: u32 = 40;

/// Search the apps' indexes for `text`: the indexed hits now (late ones come by signal, see
/// [`forward_late_hits`]).
pub(super) async fn search(
    connection: Connection,
    text_asked: String,
    generation: Generation,
    events: UnboundedSender<CompanionEvent>,
) {
    let ask = SearchAsk {
        text: text_asked,
        scope: SearchScope::Everything,
        generation: SearchGeneration(generation.0),
    };
    let got = intents(&connection).search(ask).await;
    let event = match got {
        Ok(hits) => CompanionEvent::ThingsFound(generation, hits),
        Err(error) => {
            eprintln!("sill: companion: searching: {error}");
            CompanionEvent::ThingsFailed(generation)
        }
    };
    let _ = events.send(event);
}

/// What the installed manifests say: the kinds of thing the apps open in their own windows, and
/// the apps a helper may be given.
async fn declared(
    connection: &Connection,
) -> Result<(BTreeSet<Opener>, BTreeSet<AppName>), CallError> {
    intents(connection)
        .manifests()
        .await
        .map(|all| (openers_of(&all), offered_apps(&all)))
        .map_err(CallError::from)
}

/// Read the undo journal, newest first.
pub(super) async fn journal(connection: &Connection) -> Result<Vec<UndoEntry>, CallError> {
    let filter = JournalFilter {
        run: None,
        session: None,
        limit: Count(JOURNAL_ROWS),
    };
    intents(connection)
        .journal(filter)
        .await
        .map_err(CallError::from)
}

/// Undo `scope`: one row, or a run's or a task's. The answer is the journal read again.
pub(super) async fn undo(connection: &Connection, scope: UndoScope) -> Result<(), CallError> {
    let client = intents(connection);
    match scope {
        UndoScope::Entry(entry) => client
            .undo(entry)
            .await?
            .map_err(|fault| CallError::Declined(format!("{fault:?}"))),
        scope @ (UndoScope::Run(_) | UndoScope::Task(_)) => client
            .undo_all(scope)
            .await
            .map(|_| ())
            .map_err(CallError::from),
    }
}

/// Perform `call` as the person asked, on no session.
pub(super) async fn perform(connection: &Connection, call: CallRequest) -> Result<(), CallError> {
    intents(connection)
        .perform(call, None, None)
        .await?
        .map(|_| ())
        .map_err(|refusal| CallError::Declined(format!("{refusal:?}")))
}

/// Ask the router to put `session` in `mode`, and say what it came to: the mode now, or that an
/// administrator holds every conversation in Manual, or that the switch did not reach it.
pub(super) async fn set_mode(connection: &Connection, session: SessionId, mode: Mode) -> ModeMsg {
    match intents(connection)
        .session_mode(session.clone(), mode)
        .await
    {
        Ok(()) => ModeMsg::Changed { session, mode },
        Err(ClientError::Refused(WireRefusal::Locked)) => ModeMsg::Locked,
        Err(error) => {
            eprintln!("sill: companion: switching the mode: {error}");
            ModeMsg::NotApplied(mode)
        }
    }
}

/// A mode as the router's signal names it.
fn mode_of(word: &str) -> Option<Mode> {
    serde_json::from_value(serde_json::Value::String(word.to_owned())).ok()
}

/// What intentd says on its own while it is on the bus: the hits an app answered late, the
/// journal's changes and the apps' manifests. The streams are subscribed before the journal and
/// the manifests are first read, so the first journal the service publishes says they are up.
/// Ends when the bus or the receiver goes.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    let (Ok(search), Ok(control), Ok(registry), Ok(sessions)) = (
        SearchProxy::new(&connection).await,
        docket_dbus::ControlProxy::new(&connection).await,
        RegistryProxy::new(&connection).await,
        SessionProxy::new(&connection).await,
    ) else {
        return std::future::pending().await;
    };
    let (Ok(mut hits), Ok(mut changes), Ok(mut manifests), Ok(mut modes)) = (
        search.receive_hits().await,
        control.receive_journal_changed().await,
        registry.receive_manifest_changed().await,
        sessions.receive_mode_changed().await,
    ) else {
        return std::future::pending().await;
    };
    if let Ok((found, apps)) = declared(&connection).await
        && (events.send(CompanionEvent::Openers(found)).is_err()
            || events
                .send(CompanionEvent::Helpers(Box::new(HelpersMsg::AppsOffered(
                    apps,
                ))))
                .is_err())
    {
        return;
    }
    if let Ok(rows) = journal(&connection).await
        && events.send(CompanionEvent::Journal(rows)).is_err()
    {
        return;
    }
    loop {
        let sent = tokio::select! {
            Some(signal) = next(&mut hits) => {
                let Ok(args) = signal.args() else { continue };
                let Ok(found) = from_json::<Vec<Hit>>(args.hits()) else { continue };
                events.send(CompanionEvent::ThingsLate(Generation(*args.generation()), found))
            }
            Some(_) = next(&mut changes) => match journal(&connection).await {
                Ok(rows) => events.send(CompanionEvent::Journal(rows)),
                Err(_) => continue,
            },
            Some(_) = next(&mut manifests) => match declared(&connection).await {
                Ok((found, apps)) => events
                    .send(CompanionEvent::Openers(found))
                    .and_then(|()| events.send(CompanionEvent::Helpers(Box::new(HelpersMsg::AppsOffered(apps))))),
                Err(_) => continue,
            },
            Some(signal) = next(&mut modes) => {
                let Ok(args) = signal.args() else { continue };
                let (Ok(session), Some(mode)) =
                    (SessionId::parse(args.session()), mode_of(args.mode()))
                else {
                    continue;
                };
                events.send(CompanionEvent::Mode(ModeMsg::Changed { session, mode }))
            }
            else => return,
        };
        if sent.is_err() {
            return;
        }
    }
}

/// The text the router shows for `handle` of `session`, for the screen alone: it goes into the
/// answer's card and nowhere else, never back to a model. The router labels each handle, and
/// what is not the person's own words comes back quoted ([`shown`]); a router without the
/// labelled reading, or one that does not give it to the shell, is read the old way and all of
/// it is quoted.
pub(super) async fn display(
    connection: &Connection,
    session: SessionId,
    handle: Handle,
) -> Result<String, CallError> {
    let client = intents(connection);
    match client
        .session_display_labelled(session.clone(), handle)
        .await
    {
        Ok(labelled) => Ok(shown::text_of(labelled)),
        Err(error) if shown::lacks_labels(&error) => client
            .session_display(session, handle)
            .await
            .map(|plain| shown::quoted(&plain))
            .map_err(CallError::from),
        Err(error) => Err(error.into()),
    }
}
