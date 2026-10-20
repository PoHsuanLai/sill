//! Taking the bus: one connection, the object, then the name.

use sill_model::work_areas::{WorkAreas, WorkTrouble};
use tokio::sync::watch;
use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use super::object::OutputsObject;
use crate::env::{Bus, ConnectError};
use crate::work_areas::{BUS_NAME, PATH};

/// Connect to `bus`, export the object and claim `org.quire.Outputs1` (never queued: a name
/// someone else owns is `NameTaken`, not a wait).
pub(super) async fn take(
    bus: &Bus,
    state: watch::Receiver<WorkAreas>,
) -> Result<Connection, WorkTrouble> {
    let connection = match bus.try_connect().await {
        Ok(connection) => connection,
        Err(ConnectError::NoBus(_)) => return Err(WorkTrouble::NoBus),
        Err(ConnectError::Refused(why)) => return Err(WorkTrouble::Refused(why)),
    };
    connection
        .object_server()
        .at(PATH, OutputsObject { state })
        .await
        .map_err(|error| WorkTrouble::Refused(error.to_string()))?;
    let reply = connection
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await
        .map_err(|error| trouble(error.to_string()))?;
    match reply {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(connection),
        RequestNameReply::InQueue | RequestNameReply::Exists => Err(WorkTrouble::NameTaken),
    }
}

/// A name that is taken says so in its text (zbus's "already taken"); anything else is the bus
/// refusing.
fn trouble(text: String) -> WorkTrouble {
    if text.contains("taken") {
        WorkTrouble::NameTaken
    } else {
        WorkTrouble::Refused(text)
    }
}
