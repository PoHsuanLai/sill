//! Taking the bus: one connection, the object, then the name.

use porter_dbus::{SHEET_BUS, SHEET_PATH};
use sill_model::accounts::sheet::SheetTrouble;
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use super::objects::{Call, SheetObject};
use crate::callers::CallerSource;
use crate::env::{Bus, ConnectError};

/// Connect to `bus`, export the sheet and claim `org.quire.AccountsSheet1` (never queued: a name
/// someone else owns is `NameTaken`, not a wait).
pub(super) async fn take(
    bus: &Bus,
    callers: &dyn CallerSource,
    calls: UnboundedSender<Call>,
) -> Result<Connection, SheetTrouble> {
    let connection = match bus.try_connect().await {
        Ok(connection) => connection,
        Err(ConnectError::NoBus(_)) => return Err(SheetTrouble::NoBus),
        Err(ConnectError::Refused(why)) => return Err(SheetTrouble::Refused(why)),
    };
    let peers = callers.peers_on(connection.clone());
    connection
        .object_server()
        .at(SHEET_PATH, SheetObject { calls, peers })
        .await
        .map_err(|error| SheetTrouble::Refused(error.to_string()))?;
    let reply = connection
        .request_name_with_flags(SHEET_BUS, RequestNameFlags::DoNotQueue.into())
        .await
        .map_err(|error| trouble(error.to_string()))?;
    match reply {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(connection),
        RequestNameReply::InQueue | RequestNameReply::Exists => Err(SheetTrouble::NameTaken),
    }
}

/// A name that is taken says so in its text (zbus's "already taken"); anything else is the bus
/// refusing.
fn trouble(text: String) -> SheetTrouble {
    if text.contains("taken") {
        SheetTrouble::NameTaken
    } else {
        SheetTrouble::Refused(text)
    }
}
