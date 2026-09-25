//! The Battery module: the level and time left (the bar's `battery_lines`), then the power
//! profile's [`super::PowerProfileControl`]. Without a battery (a desktop) only the control
//! shows; without either the module is absent. Reads `use_battery` and `use_power_profile`.

use dioxus::prelude::*;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    todo!("M5 fill: control-center (c)")
}
