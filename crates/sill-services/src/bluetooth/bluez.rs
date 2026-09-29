//! Hand-written `zbus` proxies for the BlueZ interfaces this service reads and calls
//! (not `bluer` or `bluez-async`, both of which run on the `dbus` crate and link libdbus, a second
//! D-Bus stack beside `zbus`). Each trait names only the methods and properties [`super::live`]
//! actually uses; BlueZ's own interfaces have more.

use zbus::proxy;

/// `org.bluez.Adapter1` (path `/org/bluez/hciN`).
#[proxy(interface = "org.bluez.Adapter1", default_service = "org.bluez")]
pub(super) trait Adapter1 {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

/// `org.bluez.Device1` (path `/org/bluez/hciN/dev_XX_XX_XX_XX_XX_XX`).
#[proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
pub(super) trait Device1 {
    fn connect(&self) -> zbus::Result<()>;

    fn disconnect(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
}

/// `org.bluez.Battery1`, present on a device's own path only once BlueZ has a reading for it.
#[proxy(interface = "org.bluez.Battery1", default_service = "org.bluez")]
pub(super) trait Battery1 {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}
