//! Bluetooth's state and its one reducer.

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Presence};

/// The first BlueZ adapter and its devices, as the control center draws them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bluetooth {
    /// Whether BlueZ answered with an adapter at all.
    pub presence: Presence,
    /// `Adapter1.Powered`.
    pub power: BtPower,
    /// Every device BlueZ knows under the adapter, in the order BlueZ reported them.
    pub devices: Vec<BtDevice>,
}

/// The adapter's radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtPower {
    #[default]
    Off,
    On,
}

/// A device's `Address` (`AA:BB:CC:DD:EE:FF`): its identity across reconnects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BtAddress(pub String);

/// One device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtDevice {
    pub address: BtAddress,
    /// `Alias`: the user's name for it, which BlueZ falls back to the device's own `Name` and
    /// then its address for.
    pub name: String,
    /// From `Device1.Icon` ([`super::device_kind`]).
    pub kind: BtKind,
    pub connection: BtConnection,
    pub pairing: BtPairing,
    /// `Battery1.Percentage`, where the device reports one.
    pub battery: BtBattery,
}

/// What sort of device it is: which glyph its row draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtKind {
    Headphones,
    Headset,
    Speaker,
    Keyboard,
    Mouse,
    Gamepad,
    Phone,
    Computer,
    #[default]
    Other,
}

/// `Device1.Connected`, and a connect under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtConnection {
    #[default]
    Disconnected,
    /// A `Connect` was sent and has not answered yet.
    Connecting,
    Connected,
}

/// `Device1.Paired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtPairing {
    #[default]
    Unpaired,
    Paired,
}

/// A device's battery, where it has `org.bluez.Battery1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BtBattery {
    #[default]
    Unknown,
    Level(Percent),
}

/// One change BlueZ reported (or a command's own progress).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BluetoothEvent {
    /// The adapter appeared, or its radio changed.
    Adapter { power: BtPower },
    /// A device appeared, or any of its properties changed: the whole device, replacing the one
    /// with its address.
    Device(BtDevice),
    /// A device's connection changed (including `Connecting` when a connect is sent).
    Connection {
        address: BtAddress,
        connection: BtConnection,
    },
    /// A device's battery changed or went away.
    Battery {
        address: BtAddress,
        battery: BtBattery,
    },
    /// BlueZ dropped a device.
    DeviceRemoved(BtAddress),
    /// The adapter went away, or BlueZ left the bus.
    Gone,
}

/// `bluetooth` after `event`. A change for an address that is not listed is dropped: the
/// `Device` event that introduces a device always comes first.
pub fn reduce_bluetooth(bluetooth: Bluetooth, event: BluetoothEvent) -> Bluetooth {
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
        BluetoothEvent::Gone => Bluetooth::default(),
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

/// The devices the control center lists, in order: connected first, then the other paired
/// devices, then the rest; by name within each group. Unpaired devices are listed only while
/// the radio is on.
pub fn listed_devices(bluetooth: &Bluetooth) -> Vec<&BtDevice> {
    let mut devices: Vec<&BtDevice> = bluetooth
        .devices
        .iter()
        .filter(|d| bluetooth.power == BtPower::On || d.pairing == BtPairing::Paired)
        .collect();
    devices.sort_by(|a, b| {
        group_of(a)
            .cmp(&group_of(b))
            .then_with(|| a.name.cmp(&b.name))
    });
    devices
}

/// A device's sort group: connected first, then paired, then the rest.
fn group_of(device: &BtDevice) -> u8 {
    if device.connection == BtConnection::Connected {
        0
    } else if device.pairing == BtPairing::Paired {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Bluetooth, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
        BtPower, listed_devices, reduce_bluetooth,
    };
    use crate::units::{Percent, Presence};

    fn device(address: &str, name: &str, connection: BtConnection, pairing: BtPairing) -> BtDevice {
        BtDevice {
            address: BtAddress(address.to_owned()),
            name: name.to_owned(),
            kind: BtKind::Headphones,
            connection,
            pairing,
            battery: BtBattery::Unknown,
        }
    }

    fn on(devices: Vec<BtDevice>) -> Bluetooth {
        Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices,
        }
    }

    fn address(text: &str) -> BtAddress {
        BtAddress(text.to_owned())
    }

    #[test]
    fn events_reduce_to_the_expected_bluetooth() {
        use BtConnection::{Connected, Connecting, Disconnected};
        use BtPairing::Paired;
        let buds = || device("AA", "Buds", Disconnected, Paired);
        let cases: Vec<(&str, Bluetooth, BluetoothEvent, Bluetooth)> = vec![
            (
                "the adapter appears powered",
                Bluetooth::default(),
                BluetoothEvent::Adapter { power: BtPower::On },
                on(Vec::new()),
            ),
            (
                "a device is added",
                on(Vec::new()),
                BluetoothEvent::Device(buds()),
                on(vec![buds()]),
            ),
            (
                "a device is replaced by address",
                on(vec![buds()]),
                BluetoothEvent::Device(device("AA", "My Buds", Disconnected, Paired)),
                on(vec![device("AA", "My Buds", Disconnected, Paired)]),
            ),
            (
                "a connect under way",
                on(vec![buds()]),
                BluetoothEvent::Connection {
                    address: address("AA"),
                    connection: Connecting,
                },
                on(vec![device("AA", "Buds", Connecting, Paired)]),
            ),
            (
                "a battery level arrives",
                on(vec![device("AA", "Buds", Connected, Paired)]),
                BluetoothEvent::Battery {
                    address: address("AA"),
                    battery: BtBattery::Level(Percent(80)),
                },
                on(vec![BtDevice {
                    battery: BtBattery::Level(Percent(80)),
                    ..device("AA", "Buds", Connected, Paired)
                }]),
            ),
            (
                "a change for an unknown address is dropped",
                on(vec![buds()]),
                BluetoothEvent::Connection {
                    address: address("ZZ"),
                    connection: Connected,
                },
                on(vec![buds()]),
            ),
            (
                "a device is removed",
                on(vec![buds()]),
                BluetoothEvent::DeviceRemoved(address("AA")),
                on(Vec::new()),
            ),
            (
                "the adapter goes",
                on(vec![buds()]),
                BluetoothEvent::Gone,
                Bluetooth::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_bluetooth(before, event), want, "{name}");
        }
    }

    #[test]
    fn connected_then_paired_then_the_rest_by_name() {
        use BtConnection::{Connected, Disconnected};
        use BtPairing::{Paired, Unpaired};
        let all = on(vec![
            device("1", "Zeta", Disconnected, Unpaired),
            device("2", "Mouse", Disconnected, Paired),
            device("3", "Buds", Connected, Paired),
            device("4", "Alpha", Disconnected, Paired),
        ]);
        let names = |b: &Bluetooth| -> Vec<String> {
            listed_devices(b).iter().map(|d| d.name.clone()).collect()
        };
        assert_eq!(names(&all), ["Buds", "Alpha", "Mouse", "Zeta"]);
        let off = Bluetooth {
            power: BtPower::Off,
            ..all
        };
        assert_eq!(
            names(&off),
            ["Buds", "Alpha", "Mouse"],
            "no strangers while off"
        );
    }
}
