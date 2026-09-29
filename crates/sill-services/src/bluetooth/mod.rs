//! Bluetooth: BlueZ's first adapter and its devices over the system bus (design/20 §1.5 and
//! design/13 §13.3.7: the control center's Bluetooth module, its device list and the devices'
//! batteries).
//!
//! BlueZ is spoken through hand-written `zbus` proxies (`org.bluez.Adapter1`, `Device1`,
//! `Battery1` and `org.freedesktop.DBus.ObjectManager`), not `bluer` or `bluez-async`: both of
//! those run on the `dbus` crate and link libdbus, a second D-Bus stack beside the `zbus` every
//! other service here uses, for four interfaces and a dozen properties.

mod bluez;
mod kind;
mod live;
mod model;

pub use kind::device_kind;
pub use model::{
    Bluetooth, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
    BtPower, listed_devices, reduce_bluetooth,
};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the Bluetooth service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothCommand {
    /// Turn the adapter's radio on or off (`Adapter1.Powered`).
    Power(BtPower),
    /// Connect a paired device (`Device1.Connect`).
    Connect(BtAddress),
    /// Disconnect a device (`Device1.Disconnect`).
    Disconnect(BtAddress),
    /// Re-read the adapter and every device (after a resume).
    Refresh,
}

/// Start the Bluetooth task on `rt`. Without BlueZ or an adapter the receiver stays at
/// `Bluetooth::default()` (absent), and the control center shows the module unavailable.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Bluetooth>, Control<BluetoothCommand>) {
    let (tx, rx) = watch::channel(Bluetooth::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
