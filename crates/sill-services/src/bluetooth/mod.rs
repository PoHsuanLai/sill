//! Bluetooth: BlueZ's first adapter and its devices over the system bus (design/20 §1.5 and
//! design/13 §13.3.7: the control center's Bluetooth module, its device list and the devices'
//! batteries).
//!
//! BlueZ is spoken through hand-written `zbus` proxies (`org.bluez.Adapter1`, `Device1`,
//! `Battery1` and `org.freedesktop.DBus.ObjectManager`), not `bluer` or `bluez-async`: both of
//! those run on the `dbus` crate and link libdbus, a second D-Bus stack beside the `zbus` every
//! other service here uses, for four interfaces and a dozen properties.

mod backend;

use sill_model::bluetooth::{Bluetooth, BluetoothCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

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
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
