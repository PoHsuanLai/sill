//! `org.freedesktop.NetworkManager.Connection.Active`.

use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
pub(crate) trait ActiveConnection {
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// The connection type, such as `802-11-wireless` (D-Bus property `Type`).
    #[zbus(property, name = "Type")]
    fn kind(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}
