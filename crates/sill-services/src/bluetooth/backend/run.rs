//! BlueZ over the system bus: seeds the adapter and its devices from the object manager, then
//! feeds every `InterfacesAdded`, `InterfacesRemoved` and property change through
//! [`sill_model::bluetooth::step`], and carries out [`BluetoothCommand`]s.
//!
//! `tokio::select!` needs a fixed set of branches, but the set of watched devices changes at
//! runtime, so each adapter/device/battery gets its own spawned task that forwards
//! [`BluetoothEvent`]s through one internal channel the main loop selects on alongside the
//! object manager's own `InterfacesAdded`/`InterfacesRemoved` streams and [`BluetoothCommand`]s.
//! A device or battery task stops when its object manager entry disappears (a `oneshot` fired
//! from the main loop); the adapter's does not, since BlueZ keeps it for the session.

use std::collections::HashMap;

use sill_model::bluetooth::{
    Bluetooth, BluetoothCommand, BluetoothEvent, BtAddress, BtBattery, BtDevice, step,
};
use tokio::sync::{mpsc, oneshot, watch};
use zbus::Connection;
use zbus::fdo::{DBusProxy, InterfacesAddedStream, InterfacesRemovedStream, ObjectManagerProxy};
use zbus::zvariant::OwnedObjectPath;

use super::devices::{
    seed_adapter, seed_battery, seed_device, watch_adapter, watch_battery, watch_device,
};
use super::pairing::{run_connect, run_disconnect, run_power};
use crate::env::Bus;
use crate::error::ServiceError;
use crate::stream::{next, next_opt};

const SERVICE: &str = "org.bluez";

const ADAPTER_IFACE: &str = "org.bluez.Adapter1";

const DEVICE_IFACE: &str = "org.bluez.Device1";

const BATTERY_IFACE: &str = "org.bluez.Battery1";

/// What the task tracks beyond the published [`Bluetooth`]: enough to route a
/// [`BluetoothCommand`] to the right object and to stop a watcher whose object went away.
#[derive(Default)]
pub(crate) struct Tracked {
    pub(crate) adapter_path: Option<OwnedObjectPath>,
    pub(crate) adapter_stop: Option<oneshot::Sender<()>>,
    pub(crate) device_path: HashMap<BtAddress, OwnedObjectPath>,
    pub(crate) device_stop: HashMap<OwnedObjectPath, oneshot::Sender<()>>,
    pub(crate) battery_stop: HashMap<OwnedObjectPath, oneshot::Sender<()>>,
}

impl Tracked {
    pub(crate) fn address_of(&self, path: &OwnedObjectPath) -> Option<BtAddress> {
        self.device_path
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(address, _)| address.clone())
    }

    pub(crate) fn stop_device(&mut self, path: &OwnedObjectPath) {
        if let Some(stop) = self.device_stop.remove(path) {
            let _ = stop.send(());
        }
        if let Some(stop) = self.battery_stop.remove(path) {
            let _ = stop.send(());
        }
    }
}

/// Runs until `commands` closes. Without a bus, BlueZ or an adapter, `tx` stays at its default.
/// BlueZ may start after sill or restart under it: a `NameOwnerChanged` for `org.bluez` rebuilds
/// the object manager's streams and re-seeds when the name gains an owner, and publishes
/// `Gone` (stopping every watcher) when it loses it.
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<Bluetooth>,
    mut commands: mpsc::UnboundedReceiver<BluetoothCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    let dbus = DBusProxy::new(&connection).await?;
    let mut owner_changes = dbus
        .receive_name_owner_changed_with_args(&[(0, SERVICE)])
        .await?;

    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel::<BluetoothEvent>();
    let mut tracked = Tracked::default();
    let mut live = attach(&connection, &tx, &internal_tx, &mut tracked).await;

    loop {
        tokio::select! {
            Some(signal) = next(&mut owner_changes) => {
                let Ok(args) = signal.args() else { continue };
                if args.new_owner().is_some() {
                    live = attach(&connection, &tx, &internal_tx, &mut tracked).await;
                } else {
                    live = Live::default();
                    apply(&tx, BluetoothEvent::Gone);
                    tracked = Tracked::default();
                }
            }
            Some(signal) = next_opt(&mut live.added) => {
                if let Ok(args) = signal.args() {
                    let path = args.object_path().to_owned().into();
                    handle_added(&connection, &tx, &internal_tx, &mut tracked, path, args.interfaces_and_properties()).await;
                }
            }
            Some(signal) = next_opt(&mut live.removed) => {
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
                    if let Some(object_manager) = &live.object_manager {
                        seed(&connection, &tx, &internal_tx, &mut tracked, object_manager).await;
                    }
                }
                None => break,
            }
        }
    }
    Ok(())
}

/// The object manager and its two signal streams, built fresh each time `org.bluez` gains an
/// owner (a stream opened against a previous owner is not trusted to follow the next one).
#[derive(Default)]
struct Live {
    object_manager: Option<ObjectManagerProxy<'static>>,
    added: Option<InterfacesAddedStream>,
    removed: Option<InterfacesRemovedStream>,
}

/// Builds a [`Live`] and seeds from it, dropping every earlier watcher first; empty when the
/// proxy or its streams cannot be built (the next owner change tries again).
async fn attach(
    connection: &Connection,
    tx: &watch::Sender<Bluetooth>,
    internal_tx: &mpsc::UnboundedSender<BluetoothEvent>,
    tracked: &mut Tracked,
) -> Live {
    *tracked = Tracked::default();
    let Some(live) = open(connection).await else {
        return Live::default();
    };
    if let Some(object_manager) = &live.object_manager {
        seed(connection, tx, internal_tx, tracked, object_manager).await;
    }
    live
}

async fn open(connection: &Connection) -> Option<Live> {
    let object_manager = ObjectManagerProxy::builder(connection)
        .destination(SERVICE)
        .ok()?
        .path("/")
        .ok()?
        .build()
        .await
        .ok()?;
    let added = object_manager.receive_interfaces_added().await.ok()?;
    let removed = object_manager.receive_interfaces_removed().await.ok()?;
    Some(Live {
        object_manager: Some(object_manager),
        added: Some(added),
        removed: Some(removed),
    })
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
        tracked.adapter_stop = Some(watch_adapter(
            connection.clone(),
            adapter_path.clone(),
            internal_tx.clone(),
        ));
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
        tracked.adapter_stop = Some(watch_adapter(connection.clone(), path, internal_tx.clone()));
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

/// Reduces `event` onto the receiver's current value and publishes the result. A `Device` event
/// keeps whatever battery `tx` already has for that address, since `step` replaces
/// the whole device and [`read_device`] never knows the battery (a separate D-Bus interface).
pub(crate) fn apply(tx: &watch::Sender<Bluetooth>, event: BluetoothEvent) {
    let event = match event {
        BluetoothEvent::Device(device) => BluetoothEvent::Device(preserve_battery(tx, device)),
        other => other,
    };
    let next = step(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

pub(crate) fn preserve_battery(tx: &watch::Sender<Bluetooth>, mut device: BtDevice) -> BtDevice {
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
pub(crate) fn under(path: &OwnedObjectPath, adapter_path: &OwnedObjectPath) -> bool {
    path.as_str()
        .strip_prefix(adapter_path.as_str())
        .is_some_and(|rest| rest.starts_with('/'))
}
