//! `org.freedesktop.NetworkManager.Settings` and `.Settings.Connection`.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
pub(crate) trait Settings {
    #[zbus(property)]
    fn connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn new_connection(&self, connection: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn connection_removed(&self, connection: OwnedObjectPath) -> zbus::Result<()>;
}

/// A connection's settings, by group and key.
pub(crate) type ConnectionMap = HashMap<String, HashMap<String, OwnedValue>>;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
pub(crate) trait ConnectionSettings {
    fn get_settings(&self) -> zbus::Result<ConnectionMap>;
}
