//! BlueZ over the system bus: seeds the adapter and its devices from the object manager, then
//! feeds every `InterfacesAdded`, `InterfacesRemoved` and property change through
//! [`super::reduce_bluetooth`], and carries out [`BluetoothCommand`]s.
//!
//! `tokio::select!` needs a fixed set of branches, but the set of watched devices changes at
//! runtime, so each adapter/device/battery gets its own spawned task that forwards
//! [`BluetoothEvent`]s through one internal channel the main loop selects on alongside the
//! object manager's own `InterfacesAdded`/`InterfacesRemoved` streams and [`BluetoothCommand`]s.
//! A device or battery task stops when its object manager entry disappears (a `oneshot` fired
//! from the main loop); the adapter's does not, since BlueZ keeps it for the session.

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot, watch};
use zbus::Connection;
use zbus::fdo::ObjectManagerProxy;
use zbus::zvariant::OwnedObjectPath;

use super::BluetoothCommand;
use super::bluez::{Adapter1Proxy, Battery1Proxy, Device1Proxy};
use super::device_kind;
use super::model::{
    Bluetooth, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice, BtPairing, BtPower,
    reduce_bluetooth,
};
use crate::env::Bus;
use crate::stream::next;
use crate::units::Percent;

const SERVICE: &str = "org.bluez";
const ADAPTER_IFACE: &str = "org.bluez.Adapter1";
const DEVICE_IFACE: &str = "org.bluez.Device1";
const BATTERY_IFACE: &str = "org.bluez.Battery1";

/// What the task tracks beyond the published [`Bluetooth`]: enough to route a
/// [`BluetoothCommand`] to the right object and to stop a watcher whose object went away.
#[derive(Default)]
struct Tracked {
    adapter_path: Option<OwnedObjectPath>,
    device_path: HashMap<BtAddress, OwnedObjectPath>,
    device_stop: HashMap<OwnedObjectPath, oneshot::Sender<()>>,
    battery_stop: HashMap<OwnedObjectPath, oneshot::Sender<()>>,
}

impl Tracked {
    fn address_of(&self, path: &OwnedObjectPath) -> Option<BtAddress> {
        self.device_path
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(address, _)| address.clone())
    }

    fn stop_device(&mut self, path: &OwnedObjectPath) {
        if let Some(stop) = self.device_stop.remove(path) {
            let _ = stop.send(());
        }
        if let Some(stop) = self.battery_stop.remove(path) {
            let _ = stop.send(());
        }
    }
}

/// Runs until `commands` closes. Without a bus, BlueZ or an adapter, `tx` stays at its default.
pub(super) async fn run(
    bus: Bus,
    tx: watch::Sender<Bluetooth>,
    mut commands: mpsc::UnboundedReceiver<BluetoothCommand>,
) {
    let Ok(Some(connection)) = bus.connect().await else {
        return;
    };
    let Ok(builder) = ObjectManagerProxy::builder(&connection).destination(SERVICE) else {
        return;
    };
    let Ok(builder) = builder.path("/") else {
        return;
    };
    let Ok(object_manager) = builder.build().await else {
        return;
    };
    let Ok(mut added) = object_manager.receive_interfaces_added().await else {
        return;
    };
    let Ok(mut removed) = object_manager.receive_interfaces_removed().await else {
        return;
    };

    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel::<BluetoothEvent>();
    let mut tracked = Tracked::default();
    seed(
        &connection,
        &tx,
        &internal_tx,
        &mut tracked,
        &object_manager,
    )
    .await;

    loop {
        tokio::select! {
            Some(signal) = next(&mut added) => {
                if let Ok(args) = signal.args() {
                    let path = args.object_path().to_owned().into();
                    handle_added(&connection, &tx, &internal_tx, &mut tracked, path, args.interfaces_and_properties()).await;
                }
            }
            Some(signal) = next(&mut removed) => {
                if let Ok(args) = signal.args() {
                    let path = args.object_path().to_owned().into();
                    handle_removed(&tx, &mut tracked, path, args.interfaces());
                }
            }
            Some(event) = internal_rx.recv() => {
                apply(&tx, event);
            }
            command = commands.recv() => match command {
                Some(BluetoothCommand::Power(power)) => run_power(&connection, &tracked, power),
                Some(BluetoothCommand::Connect(address)) => {
                    run_connect(&connection, &tx, &internal_tx, &tracked, address);
                }
                Some(BluetoothCommand::Disconnect(address)) => {
                    run_disconnect(&connection, &tracked, address);
                }
                Some(BluetoothCommand::Refresh) => {
                    seed(&connection, &tx, &internal_tx, &mut tracked, &object_manager).await;
                }
                None => break,
            }
        }
    }
}

/// Re-reads the adapter and every device under it from a fresh `GetManagedObjects`, seeding or
/// updating `tx` and (re)starting watchers for anything not already tracked. A device tracked
/// before this call but no longer reported is removed and its watchers stopped — this is also
/// how [`BluetoothCommand::Refresh`] behaves after a resume.
async fn seed(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
    tracked: &mut Tracked,
    object_manager: &ObjectManagerProxy<'_>,
) {
    let Ok(objects) = object_manager.get_managed_objects().await else {
        apply(tx, BluetoothEvent::Gone);
        *tracked = Tracked::default();
        return;
    };

    let adapter_path = objects
        .iter()
        .find(|(_, interfaces)| interfaces.keys().any(|i| i.as_str() == ADAPTER_IFACE))
        .map(|(path, _)| path.clone());
    let Some(adapter_path) = adapter_path else {
        apply(tx, BluetoothEvent::Gone);
        *tracked = Tracked::default();
        return;
    };
    if tracked.adapter_path.as_ref() != Some(&adapter_path) {
        watch_adapter(
            connection.clone(),
            adapter_path.clone(),
            internal_tx.clone(),
        );
    }
    tracked.adapter_path = Some(adapter_path.clone());
    seed_adapter(connection, tx, &adapter_path).await;

    let mut seen = Vec::new();
    for (path, interfaces) in &objects {
        if !interfaces.keys().any(|i| i.as_str() == DEVICE_IFACE) || !under(path, &adapter_path) {
            continue;
        }
        seen.push(path.clone());
        let Some(device) = seed_device(connection, tx, path).await else {
            continue;
        };
        tracked
            .device_path
            .insert(device.address.clone(), path.clone());
        if !tracked.device_stop.contains_key(path) {
            let stop = watch_device(connection.clone(), path.clone(), internal_tx.clone());
            tracked.device_stop.insert(path.clone(), stop);
        }
        if interfaces.keys().any(|i| i.as_str() == BATTERY_IFACE)
            && seed_battery(connection, tx, path, &device.address).await
            && !tracked.battery_stop.contains_key(path)
        {
            let stop = watch_battery(
                connection.clone(),
                path.clone(),
                device.address,
                internal_tx.clone(),
            );
            tracked.battery_stop.insert(path.clone(), stop);
        }
    }

    let gone: Vec<OwnedObjectPath> = tracked
        .device_stop
        .keys()
        .filter(|path| !seen.contains(path))
        .cloned()
        .collect();
    for path in gone {
        remove_device(tx, tracked, path);
    }
}

/// One object the object manager reported as added: the adapter (if none was known yet), a
/// device under it, or `Battery1` appearing on an already-known device.
async fn handle_added(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
    tracked: &mut Tracked,
    path: OwnedObjectPath,
    interfaces: &HashMap<zbus::names::InterfaceName<'_>, HashMap<&str, zbus::zvariant::Value<'_>>>,
) {
    let has = |iface: &str| interfaces.keys().any(|i| i.as_str() == iface);

    if tracked.adapter_path.is_none() && has(ADAPTER_IFACE) {
        tracked.adapter_path = Some(path.clone());
        seed_adapter(connection, tx, &path).await;
        watch_adapter(connection.clone(), path, internal_tx.clone());
        return;
    }
    let Some(adapter_path) = tracked.adapter_path.clone() else {
        return;
    };
    if has(DEVICE_IFACE) && under(&path, &adapter_path) {
        if let Some(device) = seed_device(connection, tx, &path).await {
            tracked
                .device_path
                .insert(device.address.clone(), path.clone());
            let stop = watch_device(connection.clone(), path.clone(), internal_tx.clone());
            tracked.device_stop.insert(path, stop);
        }
        return;
    }
    if has(BATTERY_IFACE)
        && let Some(address) = tracked.address_of(&path)
        && seed_battery(connection, tx, &path, &address).await
        && !tracked.battery_stop.contains_key(&path)
    {
        let stop = watch_battery(
            connection.clone(),
            path.clone(),
            address,
            internal_tx.clone(),
        );
        tracked.battery_stop.insert(path, stop);
    }
}

/// One object the object manager reported as removed: a device, `Battery1` leaving an
/// otherwise-still-present device, or the adapter itself.
fn handle_removed(
    tx: &watch::Sender<Bluetooth>,
    tracked: &mut Tracked,
    path: OwnedObjectPath,
    interfaces: &[zbus::names::InterfaceName<'_>],
) {
    let has = |iface: &str| interfaces.iter().any(|i| i.as_str() == iface);

    if has(DEVICE_IFACE) {
        remove_device(tx, tracked, path);
        return;
    }
    if has(BATTERY_IFACE) {
        if let Some(stop) = tracked.battery_stop.remove(&path) {
            let _ = stop.send(());
        }
        if let Some(address) = tracked.address_of(&path) {
            apply(
                tx,
                BluetoothEvent::Battery {
                    address,
                    battery: BtBattery::Unknown,
                },
            );
        }
        return;
    }
    if Some(&path) == tracked.adapter_path.as_ref() {
        apply(tx, BluetoothEvent::Gone);
        *tracked = Tracked::default();
    }
}

/// Stops `path`'s watchers, drops it from `tracked` and publishes its removal.
fn remove_device(tx: &watch::Sender<Bluetooth>, tracked: &mut Tracked, path: OwnedObjectPath) {
    tracked.stop_device(&path);
    if let Some(address) = tracked.address_of(&path) {
        tracked.device_path.remove(&address);
        apply(tx, BluetoothEvent::DeviceRemoved(address));
    }
}

fn run_power(connection: &Connection, tracked: &Tracked, power: BtPower) {
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
fn run_connect(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
    tracked: &Tracked,
    address: BtAddress,
) {
    let Some(path) = tracked.device_path.get(&address).cloned() else {
        return;
    };
    apply(
        tx,
        BluetoothEvent::Connection {
            address: address.clone(),
            connection: BtConnection::Connecting,
        },
    );
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

fn run_disconnect(connection: &Connection, tracked: &Tracked, address: BtAddress) {
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

async fn seed_adapter(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    path: &OwnedObjectPath,
) {
    if let Ok(adapter) = Adapter1Proxy::new(connection, path.clone()).await
        && let Ok(powered) = adapter.powered().await
    {
        apply(
            tx,
            BluetoothEvent::Adapter {
                power: power_of(powered),
            },
        );
    }
}

/// Reads a device's proxied fields and publishes it (its battery preserved from what `tx`
/// already knows: `Battery1` is a separate interface this function never touches). `None` if
/// the object is unreadable (BlueZ dropped it between the manager listing and now).
async fn seed_device(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    path: &OwnedObjectPath,
) -> Option<BtDevice> {
    let device = Device1Proxy::new(connection, path.clone()).await.ok()?;
    let read = read_device(&device).await?;
    apply(tx, BluetoothEvent::Device(read.clone()));
    Some(read)
}

/// `true` once `Battery1` truly answered (BlueZ can list the interface before it has a
/// reading): the caller only starts a watcher when this is `true`.
async fn seed_battery(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    path: &OwnedObjectPath,
    address: &BtAddress,
) -> bool {
    let Ok(battery) = Battery1Proxy::new(connection, path.clone()).await else {
        return false;
    };
    let Ok(percentage) = battery.percentage().await else {
        return false;
    };
    apply(
        tx,
        BluetoothEvent::Battery {
            address: address.clone(),
            battery: BtBattery::Level(Percent(u16::from(percentage))),
        },
    );
    true
}

/// Follows the adapter's `Powered` property for the rest of the session (BlueZ keeps the
/// adapter object as long as the daemon runs, so this watcher never needs stopping).
fn watch_adapter(
    connection: Connection,
    path: OwnedObjectPath,
    internal_tx: mpsc::UnboundedSender<BluetoothEvent>,
) {
    tokio::spawn(async move {
        let Ok(adapter) = Adapter1Proxy::new(&connection, path).await else {
            return;
        };
        let mut powered = adapter.receive_powered_changed().await;
        while let Some(change) = next(&mut powered).await {
            if let Ok(value) = change.get().await {
                let _ = internal_tx.send(BluetoothEvent::Adapter {
                    power: power_of(value),
                });
            }
        }
    });
}

/// Follows a device's `Alias`, `Icon`, `Paired` (a whole-device re-read and re-publish) and
/// `Connected` (a `Connection` event) until `stop` fires.
fn watch_device(
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

async fn republish(device: &Device1Proxy<'_>, internal_tx: &mpsc::UnboundedSender<BluetoothEvent>) {
    if let Some(device) = read_device(device).await {
        let _ = internal_tx.send(BluetoothEvent::Device(device));
    }
}

/// Follows a device's `Battery1.Percentage` until `stop` fires.
fn watch_battery(
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

/// Every field [`super::device_kind`] and `reduce_bluetooth`'s `Device` event need, read fresh
/// from `device`; its battery is left `Unknown` (a separate interface [`watch_battery`] and
/// [`seed_battery`] own) and [`apply`] fills it back in from what `tx` already knows.
async fn read_device(device: &Device1Proxy<'_>) -> Option<BtDevice> {
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

/// Reduces `event` onto the receiver's current value and publishes the result. A `Device` event
/// keeps whatever battery `tx` already has for that address, since `reduce_bluetooth` replaces
/// the whole device and [`read_device`] never knows the battery (a separate D-Bus interface).
fn apply(tx: &watch::Sender<Bluetooth>, event: BluetoothEvent) {
    let event = match event {
        BluetoothEvent::Device(device) => BluetoothEvent::Device(preserve_battery(tx, device)),
        other => other,
    };
    let next = reduce_bluetooth(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

fn preserve_battery(tx: &watch::Sender<Bluetooth>, mut device: BtDevice) -> BtDevice {
    if let Some(existing) = tx
        .borrow()
        .devices
        .iter()
        .find(|d| d.address == device.address)
    {
        device.battery = existing.battery;
    }
    device
}

/// Whether `path` sits under `adapter_path` (`/org/bluez/hci0/dev_...` under `/org/bluez/hci0`).
fn under(path: &OwnedObjectPath, adapter_path: &OwnedObjectPath) -> bool {
    path.as_str()
        .strip_prefix(adapter_path.as_str())
        .is_some_and(|rest| rest.starts_with('/'))
}

fn power_of(powered: bool) -> BtPower {
    if powered { BtPower::On } else { BtPower::Off }
}

fn connection_of(connected: bool) -> BtConnection {
    if connected {
        BtConnection::Connected
    } else {
        BtConnection::Disconnected
    }
}

fn pairing_of(paired: bool) -> BtPairing {
    if paired {
        BtPairing::Paired
    } else {
        BtPairing::Unpaired
    }
}
