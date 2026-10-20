//! BlueZ's adapter and devices read and followed: seeds and property watches.

use sill_model::bluetooth::{
    Bluetooth, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice, BtPairing, BtPower,
    device_kind,
};
use sill_model::units::Percent;
use tokio::sync::{mpsc, oneshot};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::bluez::{Adapter1Proxy, Battery1Proxy, Device1Proxy};
use crate::machine::Published;
use crate::stream::next;

pub(crate) async fn seed_adapter(
    connection: &Connection,
    tx: &Published<Bluetooth, BluetoothEvent>,
    path: &OwnedObjectPath,
) {
    if let Ok(adapter) = Adapter1Proxy::new(connection, path.clone()).await
        && let Ok(powered) = adapter.powered().await
    {
        tx.feed(BluetoothEvent::Adapter {
            power: power_of(powered),
        });
    }
}

/// Reads a device's proxied fields and publishes it (its battery preserved from what `tx`
/// already knows: `Battery1` is a separate interface this function never touches). `None` if
/// the object is unreadable (BlueZ dropped it between the manager listing and now).
pub(crate) async fn seed_device(
    connection: &Connection,
    tx: &Published<Bluetooth, BluetoothEvent>,
    path: &OwnedObjectPath,
) -> Option<BtDevice> {
    let device = Device1Proxy::new(connection, path.clone()).await.ok()?;
    let read = read_device(&device).await?;
    tx.feed(BluetoothEvent::Device(read.clone()));
    Some(read)
}

/// `true` once `Battery1` truly answered (BlueZ can list the interface before it has a
/// reading): the caller only starts a watcher when this is `true`.
pub(crate) async fn seed_battery(
    connection: &Connection,
    tx: &Published<Bluetooth, BluetoothEvent>,
    path: &OwnedObjectPath,
    address: &BtAddress,
) -> bool {
    let Ok(battery) = Battery1Proxy::new(connection, path.clone()).await else {
        return false;
    };
    let Ok(percentage) = battery.percentage().await else {
        return false;
    };
    tx.feed(BluetoothEvent::Battery {
        address: address.clone(),
        battery: BtBattery::Level(Percent(u16::from(percentage))),
    });
    true
}

/// Follows the adapter's `Powered` property until `stop` fires or drops (BlueZ going away).
pub(crate) fn watch_adapter(
    connection: Connection,
    path: OwnedObjectPath,
    internal_tx: mpsc::UnboundedSender<BluetoothEvent>,
) -> oneshot::Sender<()> {
    let (stop_tx, mut stop_rx) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(adapter) = Adapter1Proxy::new(&connection, path).await else {
            return;
        };
        let mut powered = adapter.receive_powered_changed().await;
        loop {
            tokio::select! {
                _ = &mut stop_rx => return,
                Some(change) = next(&mut powered) => {
                    if let Ok(value) = change.get().await {
                        let _ = internal_tx.send(BluetoothEvent::Adapter {
                            power: power_of(value),
                        });
                    }
                }
                else => return,
            }
        }
    });
    stop_tx
}

/// Follows a device's `Alias`, `Icon`, `Paired` (a whole-device re-read and re-publish) and
/// `Connected` (a `Connection` event) until `stop` fires.
pub(crate) fn watch_device(
    connection: Connection,
    path: OwnedObjectPath,
    internal_tx: mpsc::UnboundedSender<BluetoothEvent>,
) -> oneshot::Sender<()> {
    let (stop_tx, mut stop_rx) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(device) = Device1Proxy::new(&connection, path).await else {
            return;
        };
        let mut alias = device.receive_alias_changed().await;
        let mut icon = device.receive_icon_changed().await;
        let mut paired = device.receive_paired_changed().await;
        let mut connected = device.receive_connected_changed().await;
        loop {
            tokio::select! {
                _ = &mut stop_rx => return,
                Some(_) = next(&mut alias) => republish(&device, &internal_tx).await,
                Some(_) = next(&mut icon) => republish(&device, &internal_tx).await,
                Some(_) = next(&mut paired) => republish(&device, &internal_tx).await,
                Some(change) = next(&mut connected) => {
                    if let Ok(value) = change.get().await
                        && let Ok(address) = device.address().await
                    {
                        let _ = internal_tx.send(BluetoothEvent::Connection {
                            address: BtAddress(address),
                            connection: connection_of(value),
                        });
                    }
                }
            }
        }
    });
    stop_tx
}

pub(crate) async fn republish(
    device: &Device1Proxy<'_>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
) {
    if let Some(device) = read_device(device).await {
        let _ = internal_tx.send(BluetoothEvent::Device(device));
    }
}

/// Follows a device's `Battery1.Percentage` until `stop` fires.
pub(crate) fn watch_battery(
    connection: Connection,
    path: OwnedObjectPath,
    address: BtAddress,
    internal_tx: mpsc::UnboundedSender<BluetoothEvent>,
) -> oneshot::Sender<()> {
    let (stop_tx, mut stop_rx) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(battery) = Battery1Proxy::new(&connection, path).await else {
            return;
        };
        let mut percentage = battery.receive_percentage_changed().await;
        loop {
            tokio::select! {
                _ = &mut stop_rx => return,
                Some(change) = next(&mut percentage) => {
                    if let Ok(value) = change.get().await {
                        let _ = internal_tx.send(BluetoothEvent::Battery {
                            address: address.clone(),
                            battery: BtBattery::Level(Percent(u16::from(value))),
                        });
                    }
                }
            }
        }
    });
    stop_tx
}

/// Every field [`sill_model::bluetooth::device_kind`] and `step`'s `Device` event need, read fresh
/// from `device`; its battery is left `Unknown` (a separate interface [`watch_battery`] and
/// [`seed_battery`] own) and [`keeping_battery`] fills it back in from what is published.
pub(crate) async fn read_device(device: &Device1Proxy<'_>) -> Option<BtDevice> {
    let address = device.address().await.ok()?;
    let alias = device.alias().await.unwrap_or_default();
    let icon = device.icon().await.unwrap_or_default();
    let paired = device.paired().await.unwrap_or(false);
    let connected = device.connected().await.unwrap_or(false);
    Some(BtDevice {
        address: BtAddress(address),
        name: alias,
        kind: device_kind(&icon),
        connection: connection_of(connected),
        pairing: pairing_of(paired),
        battery: BtBattery::Unknown,
    })
}

pub(crate) fn power_of(powered: bool) -> BtPower {
    if powered { BtPower::On } else { BtPower::Off }
}

pub(crate) fn connection_of(connected: bool) -> BtConnection {
    if connected {
        BtConnection::Connected
    } else {
        BtConnection::Disconnected
    }
}

pub(crate) fn pairing_of(paired: bool) -> BtPairing {
    if paired {
        BtPairing::Paired
    } else {
        BtPairing::Unpaired
    }
}
