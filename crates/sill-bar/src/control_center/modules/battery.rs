//! The Battery module: the bar item's battery glyph and level in its header, one quire
//! `BatteryRing` per device (this computer, then each connected Bluetooth device that
//! reports a battery; design/26 5.2.9 G27), the time left, then the power profile's
//! [`super::PowerProfileControl`]. The center was just opened, so every ring sweeps in from empty
//! with its percentage counting in step. Without a battery or a device's (a desktop) only the
//! control shows; without either the module is absent. Reads `use_service::<BatteryService>`,
//! `use_service::<BluetoothService>`, `use_service::<PowerProfileService>` and
//! `bar.battery_low_percent`.

use dioxus::prelude::*;
use ds::{BatteryPower, BatteryState, Fraction, IconSource, LowAt, StatusState, TextLine};
use ds_shell::{BatteryRing, Device, DeviceGlyph, ModulePanel, Readout};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{Bluetooth, BtBattery, BtConnection};
use sill_model::units::Presence;

use super::PowerProfileControl;
use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use crate::bar::{BarMenu, StatusIcon};
use crate::control_center::modules::use_plate;
use sill_services::{BatteryService, BluetoothService, PowerProfileService};
use sill_settings::{ControlCenterSettings, InMenuBar};
use sill_ui_kit::hooks::{use_service, use_settings};
use sill_ui_kit::status::glyph::battery_glyph;
use sill_ui_kit::status::lines::battery_lines;
use sill_ui_kit::widgets::battery::device_of;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    let battery = use_service::<BatteryService>()();
    let bluetooth = use_service::<BluetoothService>()();
    let profiles = use_service::<PowerProfileService>()();
    let low = use_settings()().bar.battery_low_percent;
    let rings = device_rings(&battery, &bluetooth);
    if (battery.presence, profiles.presence, rings.is_empty())
        == (Presence::Absent, Presence::Absent, true)
    {
        return rsx! {};
    }
    let head = battery_glyph(&battery, low).map(|state| {
        let [_, left] = battery_lines(&battery);
        BatteryHead {
            state,
            percent: battery.level.0,
            left,
        }
    });
    let (glyph, title, trailing) = match &head {
        Some(head) => (
            Some(IconSource::Status(StatusState::Battery(head.state))),
            Some(TextLine::from("Battery")),
            Some(rsx! { "{head.percent}%" }),
        ),
        None => (None, None, None),
    };
    rsx! {
        ModulePanel { glyph, title, trailing, plate: use_plate(),
            div { class: "sill-cc-battery",
                if !rings.is_empty() {
                    div { class: "sill-cc-batteries", role: "group", "aria-label": "Device batteries",
                        for ring in rings {
                            BatteryRing {
                                key: "{ring.key}",
                                state: ring.state,
                                label: ring.label.clone(),
                                readout: Readout::Under,
                                DeviceGlyph { device: ring.device }
                            }
                        }
                    }
                }
                if let Some(head) = head {
                    span { class: "sill-cc-status", "{head.left}" }
                }
                PowerProfileControl {}
            }
        }
    }
}

/// The battery's header: its glyph's state and level, and the time left under it.
struct BatteryHead {
    state: BatteryState,
    percent: u16,
    left: String,
}

/// One device's ring: its key, its battery state, its name and its filled glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceRing {
    pub key: String,
    pub state: BatteryState,
    pub label: String,
    pub device: Device,
}

/// The rings, in order: this computer's battery when it has one, then every connected Bluetooth
/// device that reports a battery, in the devices pane's order.
pub(crate) fn device_rings(battery: &Battery, bluetooth: &Bluetooth) -> Vec<DeviceRing> {
    let computer = match battery.presence {
        Presence::Present => Some(DeviceRing {
            key: "computer".to_owned(),
            state: ring_state(battery.level.0, charging(battery.charge)),
            label: "This computer".to_owned(),
            // It has a battery, so a laptop.
            device: Device::Laptop,
        }),
        Presence::Absent => None,
    };
    let peripherals = sill_model::bluetooth::listed_devices(bluetooth)
        .into_iter()
        .filter_map(|device| match (device.connection, device.battery) {
            (BtConnection::Connected, BtBattery::Level(level)) => Some(DeviceRing {
                key: device.address.0.clone(),
                state: ring_state(level.0, BatteryPower::Battery),
                label: device.name.clone(),
                device: device_of(device.kind),
            }),
            (BtConnection::Connected, BtBattery::Unknown)
            | (BtConnection::Connecting | BtConnection::Disconnected, _) => None,
        });
    computer.into_iter().chain(peripherals).collect()
}

/// The ring's power: the bolt while charging.
fn charging(charge: Charge) -> BatteryPower {
    match charge {
        Charge::Charging => BatteryPower::Charging,
        Charge::Unknown | Charge::Discharging | Charge::Empty | Charge::Full | Charge::Idle => {
            BatteryPower::Battery
        }
    }
}

/// A ring's state: `percent` (capped at 100) on `power`.
fn ring_state(percent: u16, power: BatteryPower) -> BatteryState {
    BatteryState {
        level: Fraction(percent.min(100) * 10),
        power,
        low_at: LowAt::default(),
    }
}

/// The Battery item in the bar: the battery glyph by level and power; hidden without a battery (UPower's display device absent: a desktop).
#[component]
fn BatteryItem() -> Element {
    let battery = use_service::<BatteryService>()();
    let low = use_settings()().bar.battery_low_percent;
    let state = battery_glyph(&battery, low);
    let Some(state) = state else {
        return rsx! {};
    };
    let [level, _] = battery_lines(&battery);
    rsx! {
        StatusIcon {
            menu: BarMenu::Battery,
            icon: StatusState::Battery(state),
            tooltip: level,
        }
    }
}

/// The Battery module as the control center and the bar draw it.
pub(crate) struct BatteryCc;

impl CcModule for BatteryCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { BatteryModule {} }
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_battery
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::Battery,
            item: || rsx! { BatteryItem {} },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceRing, device_rings};
    use ds::{BatteryPower, BatteryState, Fraction, LowAt};
    use ds_shell::Device;
    use sill_model::battery::{Battery, Charge};
    use sill_model::bluetooth::{
        Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower,
    };
    use sill_model::units::{Percent, Presence};

    fn device(address: &str, connection: BtConnection, battery: BtBattery) -> BtDevice {
        BtDevice {
            address: BtAddress(address.to_owned()),
            name: address.to_owned(),
            kind: BtKind::Mouse,
            connection,
            pairing: BtPairing::Paired,
            battery,
        }
    }

    #[test]
    fn one_ring_per_device_that_reports_a_battery() {
        let laptop = Battery {
            presence: Presence::Present,
            level: Percent(93),
            charge: Charge::Charging,
            ..Battery::default()
        };
        let bluetooth = Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices: vec![
                device(
                    "mouse",
                    BtConnection::Connected,
                    BtBattery::Level(Percent(84)),
                ),
                device("pad", BtConnection::Connected, BtBattery::Unknown),
                device(
                    "old",
                    BtConnection::Disconnected,
                    BtBattery::Level(Percent(20)),
                ),
            ],
            connect: Default::default(),
        };
        let ring = |key: &str, level, power, label: &str, device| DeviceRing {
            key: key.to_owned(),
            state: BatteryState {
                level: Fraction(level),
                power,
                low_at: LowAt::default(),
            },
            label: label.to_owned(),
            device,
        };
        // (name, battery, bluetooth, rings)
        let cases = [
            (
                "a laptop and a mouse",
                laptop,
                bluetooth.clone(),
                vec![
                    ring(
                        "computer",
                        930,
                        BatteryPower::Charging,
                        "This computer",
                        Device::Laptop,
                    ),
                    ring("mouse", 840, BatteryPower::Battery, "mouse", Device::Mouse),
                ],
            ),
            (
                "a desktop and a mouse",
                Battery::default(),
                bluetooth,
                vec![ring(
                    "mouse",
                    840,
                    BatteryPower::Battery,
                    "mouse",
                    Device::Mouse,
                )],
            ),
            (
                "nothing",
                Battery::default(),
                Bluetooth::default(),
                Vec::new(),
            ),
        ];
        for (name, battery, bluetooth, want) in cases {
            assert_eq!(device_rings(&battery, &bluetooth), want, "{name}");
        }
    }
}
