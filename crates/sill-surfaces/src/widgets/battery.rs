//! The Battery widget (M10 lane b): this machine's battery (UPower) and every connected
//! Bluetooth device that reports one (BlueZ `Battery1`), each a level ring (quire's `LevelRing`,
//! Q183) with its glyph inside and its percentage under it, as macOS's Batteries widget. Nothing
//! to show (a desktop with no battery and no devices): the widget says "No batteries".

use dioxus::prelude::*;
use ds::{Bumped, Fraction, Glyph, Icon, IconSize, LevelRing, RingMark};
use sill_services::{
    Battery, Bluetooth, BtBattery, BtConnection, BtKind, Charge, Percent, Presence,
};

use super::WidgetHost;
use crate::hooks::{use_battery, use_bluetooth};
use crate::surfaces::bar::status::battery_icon;
use crate::surfaces::control_center::pages::bluetooth_devices::device_glyph;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_battery.css");

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

/// A row's glyph: the bar's battery glyph for this machine, the device's kind for the rest.
fn row_glyph(row: &BatteryRow) -> Icon {
    match row.owner {
        BatteryOwner::ThisMachine => battery_icon(&Battery {
            presence: Presence::Present,
            level: row.level,
            charge: row.charge,
            ..Battery::default()
        }),
        BatteryOwner::Device(kind) => device_glyph(kind),
    }
}

/// A row's ring: its level in permille, and the bolt while it charges.
fn ring_of(row: &BatteryRow) -> (Fraction, RingMark) {
    let level = Fraction(row.level.0.min(100) * 10);
    let mark = match row.charge {
        Charge::Charging => RingMark::Charging,
        _ => RingMark::Plain,
    };
    (level, mark)
}

/// What the ring tells assistive technology: "desk, 81%, charging".
fn ring_label(row: &BatteryRow) -> String {
    match row.charge {
        Charge::Charging => format!("{}, {}%, charging", row.name, row.level.0),
        _ => format!("{}, {}%", row.name, row.level.0),
    }
}

/// The batteries, as macOS's Batteries widget: one quire `LevelRing` each (the level, a bolt
/// while charging, the device's glyph inside) over its percentage, which bumps on a change.
#[component]
pub fn BatteryWidget(host: WidgetHost) -> Element {
    let battery = use_battery()();
    let bluetooth = use_bluetooth()();
    let machine = use_hook(|| {
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        machine_name(&host)
    });
    let rows = battery_rows(&battery, &bluetooth, &machine);
    rsx! {
        div { class: "sill-widget-battery", "data-host": host.slug(),
            if rows.is_empty() {
                span { class: "sill-widget-quiet", "No batteries" }
            }
            for row in rows {
                BatteryCell { key: "{row.name}", row }
            }
        }
    }
}

/// One battery: its ring over its percentage.
#[component]
fn BatteryCell(row: BatteryRow) -> Element {
    let (level, mark) = ring_of(&row);
    let percent = row.level.0;
    rsx! {
        div { class: "sill-widget-battery-cell",
            LevelRing { level, mark, label: ring_label(&row),
                Glyph { icon: row_glyph(&row), size: IconSize::Base }
            }
            span { class: "sill-widget-battery-level",
                Bumped { on: percent, "{percent}%" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sill_services::{
        Battery, Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
        Charge, Percent, Presence,
    };

    use ds::{Fraction, RingMark};

    use super::{BatteryOwner, BatteryRow, battery_rows, machine_name, ring_label, ring_of};

    #[test]
    fn a_row_is_a_ring_in_permille_with_a_bolt_while_charging() {
        let row = |level, charge| BatteryRow {
            owner: BatteryOwner::ThisMachine,
            name: "desk".to_owned(),
            level: Percent(level),
            charge,
        };
        let cases = [
            (
                row(81, Charge::Charging),
                Fraction(810),
                RingMark::Charging,
                "desk, 81%, charging",
            ),
            (
                row(7, Charge::Discharging),
                Fraction(70),
                RingMark::Plain,
                "desk, 7%",
            ),
            (
                row(100, Charge::Full),
                Fraction(1000),
                RingMark::Plain,
                "desk, 100%",
            ),
            (
                row(0, Charge::Unknown),
                Fraction(0),
                RingMark::Plain,
                "desk, 0%",
            ),
        ];
        for (row, level, mark, label) in cases {
            assert_eq!(ring_of(&row), (level, mark), "{row:?}");
            assert_eq!(ring_label(&row), label);
        }
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
