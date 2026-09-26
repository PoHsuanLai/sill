//! The Battery widget (M10 lane b; design/23-WIDGETS.md section 4.1, the fourth pass "Flat,
//! bright, measured"): this machine's battery (UPower) and every connected Bluetooth device that
//! reports one (BlueZ `Battery1`), each a level ring (quire's `BatteryLevel`, Q183) with its
//! glyph inside. quire draws only the ring; this widget composes the rest, copying the
//! reference's own gallery ("Widget reference" page): one device alone is the ring at the top
//! left with the percentage as the card's hero figure; several are a 2 x 2 grid of rings, no
//! numbers, empty places a ring at 0; a Medium frame is a row of up to four rings, the
//! percentage under each. No header row (M25): [`crate::widgets::frame::frame_head`] gives
//! Battery none. Nothing to show (a desktop with no battery and no devices): the widget says "No
//! batteries".

use dioxus::prelude::*;
use ds::{BatteryFigure, BatteryLevel, Fraction, Glyph, Icon, IconSize, RingMark, WakeStamp};
use sill_services::{
    Battery, Bluetooth, BtBattery, BtConnection, BtKind, Charge, Percent, Presence,
};

use super::WidgetHost;
use crate::hooks::{use_battery, use_bluetooth};
use crate::surfaces::bar::status::battery_icon;
use crate::surfaces::control_center::pages::bluetooth_devices::device_glyph;
use crate::widgets::use_widget_wake;

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

/// How many rings a grid or a row holds (macOS's Batteries widget shows up to four).
const MAX_RINGS: usize = 4;

/// How the widget composes its rings for `host` with `rows` batteries, at least one
/// (design/23-WIDGETS.md section 4.1, "Composition").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BatteryLayout {
    /// A Small frame, one device: the ring at the top left, the percentage as the hero figure.
    Solo,
    /// A Small frame, several devices: a 2 x 2 grid of rings, no numbers.
    Grid,
    /// A Medium frame: a row of rings, the percentage under each.
    Row,
}

impl BatteryLayout {
    /// `host`'s layout for `rows` batteries.
    fn of(host: WidgetHost, rows: usize) -> Self {
        match (host, rows) {
            (WidgetHost::Desktop, 1) => BatteryLayout::Solo,
            (WidgetHost::Desktop, _) => BatteryLayout::Grid,
            (WidgetHost::Tile, _) => BatteryLayout::Row,
        }
    }
}

/// The batteries, laid out as the reference's own gallery composes them (quire's `BatteryLevel`
/// draws only the ring). Nothing to show: a quiet line.
#[component]
pub fn BatteryWidget(host: WidgetHost) -> Element {
    let battery = use_battery()();
    let bluetooth = use_bluetooth()();
    let machine = use_hook(|| {
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        machine_name(&host)
    });
    let rows = battery_rows(&battery, &bluetooth, &machine);
    let wake = use_widget_wake();
    rsx! {
        div { class: "sill-widget-battery", "data-host": host.slug(),
            if rows.is_empty() {
                span { class: "sill-widget-quiet", "No batteries" }
            } else {
                match BatteryLayout::of(host, rows.len()) {
                    BatteryLayout::Solo => solo(&rows[0], wake),
                    BatteryLayout::Grid => grid(&rows, wake),
                    BatteryLayout::Row => row_of(&rows, wake),
                }
            }
        }
    }
}

/// One row's ring, keyed by its name: the device's glyph inside, filled to its level, a bolt
/// while it charges, its fill replayed on each new `wake`.
fn ring(row: &BatteryRow, wake: WakeStamp) -> Element {
    let (level, mark) = ring_of(row);
    rsx! {
        BatteryLevel { key: "{row.name}", level, mark, label: ring_label(row), wake,
            Glyph { icon: row_glyph(row), size: IconSize::Base }
        }
    }
}

/// An empty place in the grid: a ring at 0, as the reference's own gallery draws one.
fn empty_ring(key: usize, wake: WakeStamp) -> Element {
    rsx! {
        BatteryLevel { key: "empty-{key}", level: Fraction(0), label: "No device", wake }
    }
}

/// One device, alone: the ring at the top left, its percentage counting up as the card's hero
/// figure.
fn solo(row: &BatteryRow, wake: WakeStamp) -> Element {
    let (level, _) = ring_of(row);
    rsx! {
        div { class: "sill-widget-battery-solo",
            {ring(row, wake)}
            span { class: "sill-widget-battery-hero", BatteryFigure { level, wake } }
        }
    }
}

/// Several devices, a Small frame: a 2 x 2 grid of rings, empty places padded to four.
fn grid(rows: &[BatteryRow], wake: WakeStamp) -> Element {
    rsx! {
        div { class: "sill-widget-battery-grid",
            for row in rows.iter().take(MAX_RINGS) {
                {ring(row, wake)}
            }
            for n in rows.len()..MAX_RINGS {
                {empty_ring(n, wake)}
            }
        }
    }
}

/// A Medium frame: a row of rings, each counting up to its percentage under it.
fn row_of(rows: &[BatteryRow], wake: WakeStamp) -> Element {
    rsx! {
        div { class: "sill-widget-battery-row",
            for row in rows.iter().take(MAX_RINGS) {
                div { key: "{row.name}", class: "sill-widget-battery-cell",
                    {ring(row, wake)}
                    span { class: "sill-widget-battery-figure",
                        BatteryFigure { level: ring_of(row).0, wake }
                    }
                }
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

    use super::{
        BatteryLayout, BatteryOwner, BatteryRow, WidgetHost, battery_rows, machine_name,
        ring_label, ring_of,
    };

    #[test]
    fn the_layout_is_solo_only_once_on_the_desktop() {
        let cases = [
            (WidgetHost::Desktop, 1, BatteryLayout::Solo),
            (WidgetHost::Desktop, 2, BatteryLayout::Grid),
            (WidgetHost::Desktop, 4, BatteryLayout::Grid),
            (WidgetHost::Tile, 1, BatteryLayout::Row),
            (WidgetHost::Tile, 4, BatteryLayout::Row),
        ];
        for (host, rows, want) in cases {
            assert_eq!(BatteryLayout::of(host, rows), want, "{host:?} {rows}");
        }
    }

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
