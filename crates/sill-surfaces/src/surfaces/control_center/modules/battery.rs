//! The Battery module: the bar item's battery glyph and level in its header, one quire
//! `DeviceBattery` ring per device (this computer, then each connected Bluetooth device that
//! reports a battery; design/26 5.2.9 G27), the time left, then the power profile's
//! [`super::PowerProfileControl`]. The center was just opened, so every ring sweeps in from
//! empty with its percentage counting in step. Without a battery or a device's (a desktop) only
//! the control shows; without either the module is absent. Reads `use_battery`,
//! `use_bluetooth`, `use_power_profile` and `bar.battery_low_percent`.

use dioxus::prelude::*;
use ds::detail::FirstShow;
use ds::{
    BatteryState, Device, DeviceBattery, DeviceGlyph, Fraction, IconSource, ModulePanel, RingMark,
    StatusState, Text,
};
use sill_services::{Battery, Bluetooth, BtBattery, BtConnection, Charge, Presence};

use super::PowerProfileControl;
use crate::hooks::{use_battery, use_bluetooth, use_power_profile, use_settings};
use crate::surfaces::bar::glyph_state::battery_glyph;
use crate::surfaces::bar::status::battery_lines;
use crate::surfaces::control_center::modules::use_plate;
use crate::surfaces::control_center::pages::bluetooth_devices::device_of;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    let battery = use_battery()();
    let bluetooth = use_bluetooth()();
    let profiles = use_power_profile()();
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
            Some(Text::from("Battery")),
            Some(rsx! { "{head.percent}%" }),
        ),
        None => (None, None, None),
    };
    rsx! {
        // The center was just opened: the fill sweeps in from empty.
        ModulePanel { glyph, first: FirstShow::Animate, title, trailing, plate: use_plate(),
            div { class: "sill-cc-battery",
                if !rings.is_empty() {
                    div { class: "sill-cc-batteries", role: "group", "aria-label": "Device batteries",
                        for ring in rings {
                            DeviceBattery {
                                key: "{ring.key}",
                                level: ring.level,
                                mark: ring.mark,
                                label: ring.label.clone(),
                                first: FirstShow::Animate,
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

/// One device's ring: its key, level, whether it is charging, its name and its filled glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceRing {
    pub key: String,
    pub level: Fraction,
    pub mark: RingMark,
    pub label: String,
    pub device: Device,
}

/// The rings, in order: this computer's battery when it has one, then every connected Bluetooth
/// device that reports a battery, in the devices pane's order.
pub(crate) fn device_rings(battery: &Battery, bluetooth: &Bluetooth) -> Vec<DeviceRing> {
    let computer = match battery.presence {
        Presence::Present => Some(DeviceRing {
            key: "computer".to_owned(),
            level: Fraction(battery.level.0.min(100) * 10),
            mark: charging(battery.charge),
            label: "This computer".to_owned(),
            // It has a battery, so a laptop.
            device: Device::Laptop,
        }),
        Presence::Absent => None,
    };
    let peripherals = sill_services::listed_devices(bluetooth)
        .into_iter()
        .filter_map(|device| match (device.connection, device.battery) {
            (BtConnection::Connected, BtBattery::Level(level)) => Some(DeviceRing {
                key: device.address.0.clone(),
                level: Fraction(level.0.min(100) * 10),
                mark: RingMark::Plain,
                label: device.name.clone(),
                device: device_of(device.kind),
            }),
            (BtConnection::Connected, BtBattery::Unknown)
            | (BtConnection::Connecting | BtConnection::Disconnected, _) => None,
        });
    computer.into_iter().chain(peripherals).collect()
}

/// The ring's mark: the bolt while charging.
fn charging(charge: Charge) -> RingMark {
    match charge {
        Charge::Charging => RingMark::Charging,
        Charge::Unknown | Charge::Discharging | Charge::Empty | Charge::Full | Charge::Idle => {
            RingMark::Plain
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceRing, device_rings};
    use ds::{Device, Fraction, RingMark};
    use sill_services::{
        Battery, Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
        BtPower, Charge, Percent, Presence,
    };

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
        let ring = |key: &str, level, mark, label: &str, device| DeviceRing {
            key: key.to_owned(),
            level: Fraction(level),
            mark,
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
                        RingMark::Charging,
                        "This computer",
                        Device::Laptop,
                    ),
                    ring("mouse", 840, RingMark::Plain, "mouse", Device::Mouse),
                ],
            ),
            (
                "a desktop and a mouse",
                Battery::default(),
                bluetooth,
                vec![ring("mouse", 840, RingMark::Plain, "mouse", Device::Mouse)],
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
