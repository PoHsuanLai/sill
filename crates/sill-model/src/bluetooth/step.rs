//! The bluetooth state's pure transition.

use super::{Bluetooth, BluetoothEvent, BtAddress, BtConnection, BtDevice, BtPower};
use crate::attempt::AttemptStep as Step;
use crate::units::Presence;

/// `bluetooth` after `event`. A change for an address that is not listed is dropped: the
/// `Device` event that introduces a device always comes first.
pub fn step(bluetooth: Bluetooth, event: BluetoothEvent) -> Bluetooth {
    let connecting = connect_step(&bluetooth, &event);
    let bluetooth = Bluetooth {
        connect: bluetooth.connect.next(connecting),
        ..bluetooth
    };
    step_devices(bluetooth, event)
}

/// What `event` means for the connect under way: a device entering `Connecting` starts one (or
/// is the same one reported again), one falling from `Connecting` to `Disconnected` is a failed
/// connect, one reaching `Connected` from `Connecting` lands it, and so does the radio going off
/// or the adapter going away.
fn connect_step(bluetooth: &Bluetooth, event: &BluetoothEvent) -> Step {
    let was = |address: &BtAddress| {
        bluetooth
            .devices
            .iter()
            .find(|d| &d.address == address)
            .map(|d| d.connection)
    };
    match event {
        BluetoothEvent::Device(device) => connection_step(was(&device.address), device.connection),
        BluetoothEvent::Connection {
            address,
            connection,
        } => connection_step(was(address), *connection),
        BluetoothEvent::Adapter {
            power: BtPower::Off,
        }
        | BluetoothEvent::Gone => Step::Land,
        BluetoothEvent::Adapter { power: BtPower::On }
        | BluetoothEvent::Battery { .. }
        | BluetoothEvent::DeviceRemoved(_) => Step::Hold,
    }
}

/// One device's connection going from `was` (`None`: not listed yet) to `now`.
fn connection_step(was: Option<BtConnection>, now: BtConnection) -> Step {
    match (was, now) {
        (_, BtConnection::Connecting) => Step::Start,
        (Some(BtConnection::Connecting), BtConnection::Disconnected) => Step::Fail,
        (Some(BtConnection::Connecting), BtConnection::Connected) => Step::Land,
        (
            None | Some(BtConnection::Disconnected | BtConnection::Connected),
            BtConnection::Disconnected | BtConnection::Connected,
        ) => Step::Hold,
    }
}

/// The devices after `event`.
fn step_devices(bluetooth: Bluetooth, event: BluetoothEvent) -> Bluetooth {
    match event {
        BluetoothEvent::Adapter { power } => Bluetooth {
            presence: Presence::Present,
            power,
            ..bluetooth
        },
        BluetoothEvent::Device(device) => {
            let mut devices = bluetooth.devices;
            match devices.iter_mut().find(|d| d.address == device.address) {
                Some(existing) => *existing = device,
                None => devices.push(device),
            }
            Bluetooth {
                devices,
                ..bluetooth
            }
        }
        BluetoothEvent::Connection {
            address,
            connection,
        } => with_device(bluetooth, &address, |device| device.connection = connection),
        BluetoothEvent::Battery { address, battery } => {
            with_device(bluetooth, &address, |device| device.battery = battery)
        }
        BluetoothEvent::DeviceRemoved(address) => {
            let mut devices = bluetooth.devices;
            devices.retain(|d| d.address != address);
            Bluetooth {
                devices,
                ..bluetooth
            }
        }
        // The stamps go on counting, so a later connect never reuses one.
        BluetoothEvent::Gone => Bluetooth {
            connect: bluetooth.connect,
            ..Bluetooth::default()
        },
    }
}

/// Applies `f` to the device at `address`, if listed; otherwise `bluetooth` is unchanged.
fn with_device(
    mut bluetooth: Bluetooth,
    address: &BtAddress,
    f: impl FnOnce(&mut BtDevice),
) -> Bluetooth {
    if let Some(device) = bluetooth.devices.iter_mut().find(|d| &d.address == address) {
        f(device);
    }
    bluetooth
}
