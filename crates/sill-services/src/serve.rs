//! Claiming a well-known bus name for the objects a service serves: connect, export the objects,
//! then ask for the name without queueing. One place decides what a refusal is, so every service
//! that serves a name says the same three things when it cannot: no bus, name taken, refused.

use std::fmt::Display;

use sill_model::serve::{ClaimError, ServeLink};
use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use crate::env::{Bus, ConnectError};

impl From<ConnectError> for ClaimError {
    fn from(error: ConnectError) -> Self {
        match error {
            ConnectError::NoBus(_) => ClaimError::NoBus,
            ConnectError::Refused(why) => ClaimError::Refused(why),
        }
    }
}

/// Connect to `bus`, let `objects` export what the name will serve, then claim `name` (never
/// queued: a name someone else owns is [`ClaimError::NameTaken`], not a wait). The objects are
/// exported first so no call can arrive at a name that has nothing behind it.
pub(crate) async fn claim(
    bus: &Bus,
    name: &str,
    objects: impl AsyncFnOnce(&Connection) -> Result<(), ClaimError>,
) -> Result<Connection, ClaimError> {
    let connection = bus.try_connect().await?;
    objects(&connection).await?;
    take_name(&connection, name).await?;
    Ok(connection)
}

/// What a claim leaves a service holding: the link to publish, and the connection when the name
/// is ours.
pub(crate) fn held(claimed: Result<Connection, ClaimError>) -> (ServeLink, Option<Connection>) {
    match claimed {
        Ok(connection) => (ServeLink::Serving, Some(connection)),
        Err(error) => (ServeLink::Unavailable(error), None),
    }
}

/// Ask for `name` on `connection` without queueing.
pub(crate) async fn take_name(connection: &Connection, name: &str) -> Result<(), ClaimError> {
    match connection
        .request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(()),
        Ok(RequestNameReply::InQueue | RequestNameReply::Exists) | Err(zbus::Error::NameTaken) => {
            Err(ClaimError::NameTaken(format!("{name} is taken")))
        }
        Err(error) => Err(refused(error)),
    }
}

/// The bus refusing something other than the name, with its words.
pub(crate) fn refused(error: impl Display) -> ClaimError {
    ClaimError::Refused(error.to_string())
}
