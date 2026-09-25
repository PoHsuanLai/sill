//! BlueZ over the system bus: seeds the adapter and its devices from the object manager, then
//! feeds every `InterfacesAdded`, `InterfacesRemoved` and property change through
//! [`super::reduce_bluetooth`], and carries out [`BluetoothCommand`]s.

use tokio::sync::{mpsc, watch};

use super::BluetoothCommand;
use super::model::Bluetooth;

/// Runs until `commands` closes. Without a bus, BlueZ or an adapter, `tx` stays at its default.
///
/// M5 freeze stub, filled by M5 agent (a) svc-session-bt (FINDINGS "M5 freeze"): until then it
/// returns at once, so the service is inert (its receiver at the default, its commands `Gone`).
pub(super) async fn run(
    tx: watch::Sender<Bluetooth>,
    commands: mpsc::UnboundedReceiver<BluetoothCommand>,
) {
    let _ = (tx, commands);
}
