//! `org.freedesktop.NetworkManager.Device.Wireless`.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, Value};

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
pub(crate) trait WirelessDevice {
    fn request_scan(&self, options: HashMap<String, Value<'_>>) -> zbus::Result<()>;

    #[zbus(property)]
    fn access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    #[zbus(signal)]
    fn access_point_added(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn access_point_removed(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;
}
