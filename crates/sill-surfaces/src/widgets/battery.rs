//! The Battery widget (M10 lane b): this machine's battery (UPower) and every connected
//! Bluetooth device that reports one (BlueZ `Battery1`), each a glyph, a name and a level (quire
//! `ProgressRing` or `Meter`, whichever quire offers for a level), as macOS's Batteries widget.
//! Nothing to show (a desktop with no battery and no devices): the widget says "No batteries".

use dioxus::prelude::*;
use sill_services::{Battery, Bluetooth, BtKind, Charge, Percent};

use super::WidgetHost;

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
    let _ = (battery, bluetooth, machine);
    todo!("M10 fill: b-widgets")
}

/// The batteries.
#[component]
pub fn BatteryWidget(host: WidgetHost) -> Element {
    let _ = host;
    rsx! {}
}

#[cfg(test)]
mod tests {
    use sill_services::{
        Battery, Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
        Charge, Percent, Presence,
    };

    use super::{BatteryOwner, BatteryRow, battery_rows};

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
    #[ignore = "M10 fill: b-widgets"]
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
