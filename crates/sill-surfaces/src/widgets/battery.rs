//! The Batteries widget's provider (design/23-WIDGETS.md section 4.1): this
//! machine's battery (UPower) and every connected Bluetooth device that reports one (BlueZ
//! `Battery1`), as quire's `BatteryEntry`. quire's `BatteryWidget` draws them (one ring with the
//! hero figure, the 2 x 2 grid, the row; each ring holds the device's filled glyph and fills on
//! the host's wake); sill only reads the services. Nothing to show (a desktop with no battery
//! and no devices): `Absent("No batteries")`.

use dioxus::prelude::*;
use ds::{BatteryCell, BatteryEntry, Device, Fraction, RingMark, WidgetCard};
use sill_model::units::{Percent, Presence};
use sill_services::{Battery, Bluetooth, BtBattery, BtConnection, BtKind, Charge};

use super::{Place, WidgetKind, widget_element_id};
use crate::hooks::use_service;
use crate::surfaces::control_center::pages::bluetooth_devices::device_of;
use crate::widgets::use_widget_wake;
use sill_services::{BatteryService, BluetoothService};

/// Whose battery a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BatteryOwner {
    ThisMachine,
    Device(BtKind),
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BatteryRow {
    pub owner: BatteryOwner,
    /// "This Mac"'s stand-in is the host name's first label, else "This Computer"; a device's
    /// name is its alias.
    pub name: String,
    pub level: Percent,
    /// This machine's charge state; a device's is `Unknown` (BlueZ does not say).
    pub charge: Charge,
}

/// This machine's row first when it has a battery, then each connected device with a level,
/// by name. `machine` is the name the machine's row takes.
pub fn battery_rows(battery: &Battery, bluetooth: &Bluetooth, machine: &str) -> Vec<BatteryRow> {
    let this = match battery.presence {
        Presence::Present => Some(BatteryRow {
            owner: BatteryOwner::ThisMachine,
            name: machine.to_owned(),
            level: battery.level,
            charge: battery.charge,
        }),
        Presence::Absent => None,
    };
    let mut devices: Vec<BatteryRow> = bluetooth
        .devices
        .iter()
        .filter(|device| device.connection == BtConnection::Connected)
        .filter_map(|device| match device.battery {
            BtBattery::Level(level) => Some(BatteryRow {
                owner: BatteryOwner::Device(device.kind),
                name: device.name.clone(),
                level,
                charge: Charge::Unknown,
            }),
            BtBattery::Unknown => None,
        })
        .collect();
    devices.sort_by_key(|row| row.name.to_lowercase());
    this.into_iter().chain(devices).collect()
}

/// The machine row's name: the host name's first label, else "This Computer".
pub(crate) fn machine_name(host: &str) -> String {
    match host
        .trim()
        .split('.')
        .next()
        .filter(|label| !label.is_empty())
    {
        Some(label) => label.to_owned(),
        None => "This Computer".to_owned(),
    }
}

/// A row as quire's cell: this machine is a laptop (it has a battery), a device its kind's
/// glyph; the level in permille, the bolt while it charges.
pub fn cell_of(row: &BatteryRow) -> BatteryCell {
    let device = match row.owner {
        BatteryOwner::ThisMachine => Device::Laptop,
        BatteryOwner::Device(kind) => device_of(kind),
    };
    let mark = match row.charge {
        Charge::Charging => RingMark::Charging,
        Charge::Unknown | Charge::Discharging | Charge::Empty | Charge::Full | Charge::Idle => {
            RingMark::Plain
        }
    };
    BatteryCell {
        name: row.name.clone(),
        device,
        level: Fraction(row.level.0.min(100) * 10),
        mark,
    }
}

/// The widget's entry for `rows`.
pub fn battery_entry(rows: &[BatteryRow]) -> BatteryEntry {
    match rows {
        [] => BatteryEntry::Absent("No batteries".to_owned()),
        rows => BatteryEntry::Devices(rows.iter().map(cell_of).collect()),
    }
}

/// The batteries on quire's card: a new entry each time a level changes.
#[component]
pub fn BatteryCard(place: Place) -> Element {
    let battery = use_service::<BatteryService>()();
    let bluetooth = use_service::<BluetoothService>()();
    let machine = use_hook(|| {
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        machine_name(&host)
    });
    let entry = battery_entry(&battery_rows(&battery, &bluetooth, &machine));
    rsx! {
        WidgetCard {
            widget: ds::BatteryWidget,
            timeline: ds::Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::Battery),
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}

#[cfg(test)]
mod tests {
    use sill_model::units::{Percent, Presence};
    use sill_services::{
        Battery, Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, Charge,
    };

    use ds::{BatteryCell, BatteryEntry, Device, Fraction, RingMark};

    use super::{BatteryOwner, BatteryRow, battery_entry, battery_rows, cell_of, machine_name};

    #[test]
    fn a_row_is_a_cell_in_permille_with_its_device_and_a_bolt_while_charging() {
        let row = |owner, level, charge| BatteryRow {
            owner,
            name: "desk".to_owned(),
            level: Percent(level),
            charge,
        };
        let cell = |device, level, mark| BatteryCell {
            name: "desk".to_owned(),
            device,
            level: Fraction(level),
            mark,
        };
        let this = BatteryOwner::ThisMachine;
        let cases = [
            (
                row(this, 81, Charge::Charging),
                cell(Device::Laptop, 810, RingMark::Charging),
            ),
            (
                row(this, 7, Charge::Discharging),
                cell(Device::Laptop, 70, RingMark::Plain),
            ),
            (
                row(this, 100, Charge::Full),
                cell(Device::Laptop, 1000, RingMark::Plain),
            ),
            (
                row(BatteryOwner::Device(BtKind::Headset), 0, Charge::Unknown),
                cell(Device::Headphones, 0, RingMark::Plain),
            ),
            (
                row(BatteryOwner::Device(BtKind::Mouse), 140, Charge::Unknown),
                cell(Device::Mouse, 1000, RingMark::Plain),
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
            BatteryEntry::Devices(vec![cell(Device::Laptop, 500, RingMark::Plain)])
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
}
