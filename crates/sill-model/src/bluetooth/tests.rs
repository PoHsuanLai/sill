use super::{
    Bluetooth, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
    BtPower, listed_devices, step,
};
use crate::attempt::{Attempt, Attempts, OpStamp};
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
        connect: Attempts::default(),
    }
}

fn stamped(bluetooth: Bluetooth, now: Attempt, minted: u32) -> Bluetooth {
    Bluetooth {
        connect: Attempts {
            now,
            minted: OpStamp(minted),
        },
        ..bluetooth
    }
}

#[test]
fn each_connect_is_stamped_once_and_each_failure_anew() {
    use Attempt::{Failed, Quiet, Running};
    use BtConnection::{Connected, Connecting, Disconnected};
    use BtPairing::Paired;
    let s = OpStamp;
    let buds = |c| on(vec![device("AA", "Buds", c, Paired)]);
    let to = |connection| BluetoothEvent::Connection {
        address: address("AA"),
        connection,
    };
    // (name, before, event, after)
    let cases = [
        (
            "a connect mints",
            stamped(buds(Disconnected), Quiet, 0),
            to(Connecting),
            stamped(buds(Connecting), Running(s(1)), 1),
        ),
        (
            "the same connect again keeps it",
            stamped(buds(Connecting), Running(s(1)), 1),
            BluetoothEvent::Device(device("AA", "Buds", Connecting, Paired)),
            stamped(buds(Connecting), Running(s(1)), 1),
        ),
        (
            "a connect that falls back fails anew",
            stamped(buds(Connecting), Running(s(1)), 1),
            to(Disconnected),
            stamped(buds(Disconnected), Failed(s(2)), 2),
        ),
        (
            "connected lands",
            stamped(buds(Connecting), Running(s(3)), 3),
            to(Connected),
            stamped(buds(Connected), Quiet, 3),
        ),
        (
            "a device dropping on its own is no failure",
            stamped(buds(Connected), Quiet, 3),
            to(Disconnected),
            stamped(buds(Disconnected), Quiet, 3),
        ),
        (
            "the radio off lands",
            stamped(buds(Connecting), Running(s(4)), 4),
            BluetoothEvent::Adapter {
                power: BtPower::Off,
            },
            Bluetooth {
                power: BtPower::Off,
                ..stamped(buds(Connecting), Quiet, 4)
            },
        ),
        (
            "gone keeps the count",
            stamped(buds(Connecting), Running(s(4)), 4),
            BluetoothEvent::Gone,
            stamped(Bluetooth::default(), Quiet, 4),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
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
            stamped(
                on(vec![device("AA", "Buds", Connecting, Paired)]),
                Attempt::Running(OpStamp(1)),
                1,
            ),
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
        assert_eq!(step(before, event), want, "{name}");
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
