//! Taking the bus: one connection, the object, then the name.

use sill_model::workspace_looks::{WorkspaceLooks, WorkspaceLooksTrouble};
use tokio::sync::watch;
use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use super::object::WorkspacesObject;
use crate::env::{Bus, ConnectError};
use crate::workspace_looks::{BUS_NAME, PATH};

/// Connect to `bus`, export the object and claim `org.quire.Workspaces1` (never queued: a name
/// someone else owns is `NameTaken`, not a wait).
pub(super) async fn take(
    bus: &Bus,
    state: watch::Receiver<WorkspaceLooks>,
) -> Result<Connection, WorkspaceLooksTrouble> {
    let connection = match bus.try_connect().await {
        Ok(connection) => connection,
        Err(ConnectError::NoBus(_)) => return Err(WorkspaceLooksTrouble::NoBus),
        Err(ConnectError::Refused(why)) => return Err(WorkspaceLooksTrouble::Refused(why)),
    };
    connection
        .object_server()
        .at(PATH, WorkspacesObject { state })
        .await
        .map_err(|error| WorkspaceLooksTrouble::Refused(error.to_string()))?;
    let reply = connection
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await
        .map_err(|error| trouble(error.to_string()))?;
    match reply {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(connection),
        RequestNameReply::InQueue | RequestNameReply::Exists => {
            Err(WorkspaceLooksTrouble::NameTaken)
        }
    }
}

/// A name that is taken says so in its text (zbus's "already taken"); anything else is the bus
/// refusing.
fn trouble(text: String) -> WorkspaceLooksTrouble {
    if text.contains("taken") {
        WorkspaceLooksTrouble::NameTaken
    } else {
        WorkspaceLooksTrouble::Refused(text)
    }
}
