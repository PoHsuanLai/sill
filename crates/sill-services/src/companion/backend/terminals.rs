//! The sessions the person started in a terminal, on `org.quire.Companion1.Terminals`: the list,
//! Bring Back, and the `Changed` signal that tells the panel to read the list again. Every body
//! is JSON of `companion-wire`'s terminal types; a refusal is the companion's own sentence and
//! the panel shows it as it is. This is the only file that names those calls.

use companion_wire::{BroughtBack, TerminalSession};
use docket_dbus::{COMPANION_BUS, CompanionTerminalsProxy};
use prov::SessionId;
use sill_model::companion::{CompanionEvent, NO_BRING_BACK, TerminalsCommand, TerminalsMsg};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use super::text::from_json;
use crate::call_error::CallError;
use crate::owner;
use crate::stream::next;

/// What the companion answered to `command`, as the message the panel takes.
pub(super) async fn carry(connection: &Connection, command: TerminalsCommand) -> TerminalsMsg {
    let proxy = match CompanionTerminalsProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(error) => return unreached(&error, command),
    };
    match command {
        TerminalsCommand::List => match read_list(&proxy).await {
            Ok(sessions) => TerminalsMsg::Listed(sessions),
            Err(why) => unreached(&why, TerminalsCommand::List),
        },
        TerminalsCommand::BringBack(session) => match bring_back(&proxy, &session).await {
            Ok(brought) => TerminalsMsg::Answered(session, brought),
            Err(why) => unreached(&why, TerminalsCommand::BringBack(session)),
        },
    }
}

async fn read_list(proxy: &CompanionTerminalsProxy<'_>) -> Result<Vec<TerminalSession>, CallError> {
    let body = proxy.sessions().await?;
    from_json(&body)
}

async fn bring_back(
    proxy: &CompanionTerminalsProxy<'_>,
    session: &SessionId,
) -> Result<BroughtBack, CallError> {
    let body = proxy.bring_back(session.as_str()).await?;
    from_json(&body)
}

/// Log what went wrong. A bring back the person asked for says it did not happen; a list that
/// could not be read changes nothing.
fn unreached(why: &dyn std::fmt::Display, command: TerminalsCommand) -> TerminalsMsg {
    eprintln!("sill: companion: terminals: {why}");
    match command {
        TerminalsCommand::List => TerminalsMsg::Unreached,
        TerminalsCommand::BringBack(session) => {
            TerminalsMsg::Answered(session, BroughtBack::Refused(NO_BRING_BACK.to_owned()))
        }
    }
}

/// The sessions in terminals while the companion is on the bus: read when it appears and each
/// time it says something changed. The signal is subscribed before the first read, so no change
/// falls between the two.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    owner::while_owned(
        &connection,
        COMPANION_BUS,
        || changes(&connection, &events),
        || {},
    )
    .await;
}

/// `Changed` as the panel's message; ends when the events have no receiver.
async fn changes(connection: &Connection, events: &UnboundedSender<CompanionEvent>) {
    let Ok(proxy) = CompanionTerminalsProxy::new(connection).await else {
        return std::future::pending().await;
    };
    let Ok(mut changed) = proxy.receive_changed().await else {
        return std::future::pending().await;
    };
    let tell = |msg| events.send(CompanionEvent::Terminals(msg));
    if tell(TerminalsMsg::Changed).is_err() {
        return;
    }
    while next(&mut changed).await.is_some() {
        if tell(TerminalsMsg::Changed).is_err() {
            return;
        }
    }
}
