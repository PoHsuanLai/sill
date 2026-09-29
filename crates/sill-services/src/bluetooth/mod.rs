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
    BtPower, listed_devices, step,
};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

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

/// BlueZ's adapter and devices as a [`Service`]. Without BlueZ or an adapter the state stays
/// at `Bluetooth::default()` (absent), and the control center shows the module unavailable.
#[derive(Debug)]
pub enum BluetoothService {}

impl Service for BluetoothService {
    const NAME: &'static str = "bluetooth";
    type State = Bluetooth;
    type Command = BluetoothCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        live::run(env.buses.system.clone(), io.state, io.commands)
    }
}
