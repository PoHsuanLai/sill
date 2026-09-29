//! Whether a well-known name is on a bus: owned now, only activatable, or missing (Tracker's
//! detection, the launcher's reveal in the file manager). Starts nothing.

use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// Whether a well-known name is on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameState {
    /// Someone holds it now.
    Owned,
    /// Nobody holds it, but a call to it would start its service.
    Activatable,
    Missing,
}

/// `name`'s state on `connection`'s bus: `NameHasOwner`, then `ListActivatableNames`. Starts
/// nothing.
pub async fn name_state(connection: &Connection, name: &str) -> zbus::Result<NameState> {
    let dbus = DBusProxy::new(connection).await?;
    if dbus.name_has_owner(BusName::try_from(name)?).await? {
        return Ok(NameState::Owned);
    }
    let activatable = dbus.list_activatable_names().await?;
    Ok(if activatable.iter().any(|n| n.as_str() == name) {
        NameState::Activatable
    } else {
        NameState::Missing
    })
}
