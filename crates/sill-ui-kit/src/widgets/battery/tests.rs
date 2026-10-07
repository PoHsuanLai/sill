use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{
    Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
};
use sill_model::units::{Percent, Presence};

use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::prelude::*;
use sill_shell_kit::battery::device_glyph::Device;
use sill_shell_kit::widget::battery::{BatteryCell, BatteryEntry};

use super::provider::machine_name;
use super::{BatteryOwner, BatteryRow, battery_entry, battery_rows, cell_of};

#[test]
fn a_row_is_a_cell_in_permille_with_its_device_and_a_bolt_while_charging() {
    let row = |owner, level, charge| BatteryRow {
        owner,
        name: "desk".to_owned(),
        level: Percent(level),
        charge,
    };
    let cell = |device, level, power| BatteryCell {
        name: "desk".to_owned(),
        device,
        state: BatteryState {
            level: Fraction(level),
            power,
            low_at: LowAt::default(),
        },
    };
    let this = BatteryOwner::ThisMachine;
    let cases = [
        (
            row(this, 81, Charge::Charging),
            cell(Device::Laptop, 810, BatteryPower::Charging),
        ),
        (
            row(this, 7, Charge::Discharging),
            cell(Device::Laptop, 70, BatteryPower::Battery),
        ),
        (
            row(this, 100, Charge::Full),
            cell(Device::Laptop, 1000, BatteryPower::Battery),
        ),
        (
            row(BatteryOwner::Device(BtKind::Headset), 0, Charge::Unknown),
            cell(Device::Headphones, 0, BatteryPower::Battery),
        ),
        (
            row(BatteryOwner::Device(BtKind::Mouse), 140, Charge::Unknown),
            cell(Device::Mouse, 1000, BatteryPower::Battery),
        ),
    ];
    for (row, want) in cases {
        assert_eq!(cell_of(&row), want, "{row:?}");
    }
    assert_eq!(
        battery_entry(&[]),
        BatteryEntry::Absent("No batteries".to_owned())
    );
    assert_eq!(
        battery_entry(&[row(this, 50, Charge::Idle)]),
        BatteryEntry::Devices(vec![cell(Device::Laptop, 500, BatteryPower::Battery)])
    );
}

#[test]
fn the_machine_is_named_by_its_hosts_first_label() {
    let cases = [
        ("desk\n", "desk"),
        ("frieren.local", "frieren"),
        ("", "This Computer"),
        (" \n", "This Computer"),
    ];
    for (host, name) in cases {
        assert_eq!(machine_name(host), name, "{host:?}");
    }
}

fn device(name: &str, kind: BtKind, connection: BtConnection, battery: BtBattery) -> BtDevice {
    BtDevice {
        address: BtAddress(format!("00:00:00:00:00:{:02}", name.len())),
        name: name.to_owned(),
        kind,
        connection,
        pairing: BtPairing::Paired,
        battery,
    }
}

#[test]
fn the_machine_then_connected_devices_with_a_level_by_name() {
    let battery = Battery {
        presence: Presence::Present,
        level: Percent(81),
        charge: Charge::Charging,
        ..Battery::default()
    };
    let bluetooth = Bluetooth {
        devices: vec![
            device(
                "Mouse",
                BtKind::Mouse,
                BtConnection::Connected,
                BtBattery::Level(Percent(40)),
            ),
            device(
                "AirPods",
                BtKind::Headphones,
                BtConnection::Connected,
                BtBattery::Level(Percent(90)),
            ),
            device(
                "Pad",
                BtKind::Gamepad,
                BtConnection::Disconnected,
                BtBattery::Level(Percent(10)),
            ),
            device(
                "Keys",
                BtKind::Keyboard,
                BtConnection::Connected,
                BtBattery::Unknown,
            ),
        ],
        ..Bluetooth::default()
    };
    let rows = battery_rows(&battery, &bluetooth, "desk");
    let row = |owner, name: &str, level, charge| BatteryRow {
        owner,
        name: name.to_owned(),
        level: Percent(level),
        charge,
    };
    assert_eq!(
        rows,
        vec![
            row(BatteryOwner::ThisMachine, "desk", 81, Charge::Charging),
            row(
                BatteryOwner::Device(BtKind::Headphones),
                "AirPods",
                90,
                Charge::Unknown
            ),
            row(
                BatteryOwner::Device(BtKind::Mouse),
                "Mouse",
                40,
                Charge::Unknown
            ),
        ]
    );
    let desktop = Battery::default();
    assert_eq!(
        battery_rows(&desktop, &Bluetooth::default(), "desk"),
        Vec::new()
    );
}
