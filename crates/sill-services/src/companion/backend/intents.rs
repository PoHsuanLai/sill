//! What the shell asks intentd as the launcher: the things apps index, the undo journal, undoing,
//! and performing an action the person picked.

use docket_client::{DbusTransport, Intents};
use docket_core::{
    CallRequest, Generation as SearchGeneration, Hit, JournalFilter, SearchAsk, SearchScope,
    UndoEntry, UndoScope,
};
use docket_dbus::SearchProxy;
use porter_core::Count;
use sill_launcher::Generation;
use sill_model::companion::CompanionEvent;
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use super::text::{from_json, text};
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

/// Read the undo journal, newest first.
pub(super) async fn journal(connection: &Connection) -> Result<Vec<UndoEntry>, String> {
    let filter = JournalFilter {
        run: None,
        session: None,
        limit: Count(JOURNAL_ROWS),
    };
    intents(connection).journal(filter).await.map_err(text)
}

/// Undo `scope`: one row, or a run's or a task's. The answer is the journal read again.
pub(super) async fn undo(connection: &Connection, scope: UndoScope) -> Result<(), String> {
    let client = intents(connection);
    match scope {
        UndoScope::Entry(entry) => client
            .undo(entry)
            .await
            .map_err(text)?
            .map_err(|fault| format!("{fault:?}")),
        scope @ (UndoScope::Run(_) | UndoScope::Task(_)) => {
            client.undo_all(scope).await.map(|_| ()).map_err(text)
        }
    }
}

/// Perform `call` as the person asked, on no session.
pub(super) async fn perform(connection: &Connection, call: CallRequest) -> Result<(), String> {
    intents(connection)
        .perform(call, None, None)
        .await
        .map_err(text)?
        .map(|_| ())
        .map_err(|refusal| format!("{refusal:?}"))
}

/// What intentd says on its own while it is on the bus: the hits an app answered late, and the
/// journal's changes. Both streams are subscribed before the journal is first read, so the first
/// journal the service publishes says they are up. Ends when the bus or the receiver goes.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    let (Ok(search), Ok(control)) = (
        SearchProxy::new(&connection).await,
        docket_dbus::ControlProxy::new(&connection).await,
    ) else {
        return std::future::pending().await;
    };
    let (Ok(mut hits), Ok(mut changes)) = (
        search.receive_hits().await,
        control.receive_journal_changed().await,
    ) else {
        return std::future::pending().await;
    };
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
            else => return,
        };
        if sent.is_err() {
            return;
        }
    }
}
