//! What the service asks of BlueZ: the radio, connecting and disconnecting a device.

use airwaves_bluez::{Adapter1Proxy, Device1Proxy};
use sill_model::bluetooth::{Bluetooth, BluetoothEvent, BtAddress, BtConnection, BtPower};
use tokio::sync::mpsc;
use zbus::Connection;

use super::run::Tracked;
use crate::machine::Published;

pub(crate) fn run_power(connection: &Connection, tracked: &Tracked, power: BtPower) {
    let Some(path) = tracked.adapter_path.clone() else {
        return;
    };
    let connection = connection.clone();
    tokio::spawn(async move {
        if let Ok(adapter) = Adapter1Proxy::new(&connection, path).await {
            let _ = adapter.set_powered(power == BtPower::On).await;
        }
    });
}

/// Publishes `Connecting` at once, then carries out the connect in the background: `Device1`'s
/// own `Connected` property change (watched by [`watch_device`]) is what confirms it, so a
/// failure here reverts the optimistic state instead of leaving it stuck.
pub(crate) fn run_connect(
    connection: &Connection,
    tx: &Published<Bluetooth, BluetoothEvent>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
    tracked: &Tracked,
    address: BtAddress,
) {
    let Some(path) = tracked.device_path.get(&address).cloned() else {
        return;
    };
    tx.feed(BluetoothEvent::Connection {
        address: address.clone(),
        connection: BtConnection::Connecting,
    });
    let connection = connection.clone();
    let internal_tx = internal_tx.clone();
    tokio::spawn(async move {
        let ok = match Device1Proxy::new(&connection, path).await {
            Ok(device) => device.connect().await.is_ok(),
            Err(_) => false,
        };
        if !ok {
            let _ = internal_tx.send(BluetoothEvent::Connection {
                address,
                connection: BtConnection::Disconnected,
            });
        }
    });
}

pub(crate) fn run_disconnect(connection: &Connection, tracked: &Tracked, address: BtAddress) {
    let Some(path) = tracked.device_path.get(&address).cloned() else {
        return;
    };
    let connection = connection.clone();
    tokio::spawn(async move {
        if let Ok(device) = Device1Proxy::new(&connection, path).await {
            let _ = device.disconnect().await;
        }
    });
}
