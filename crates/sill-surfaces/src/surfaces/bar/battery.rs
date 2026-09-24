//! The battery status item and its menu (level, time remaining).

use dioxus::prelude::*;
use shell_host::OutputInfo;
use sill_services::Presence;

use super::menus::BarMenu;
use super::status::{battery_icon, battery_lines};
use super::status_item::StatusIcon;
use crate::hooks::use_battery;

/// The battery status item and its menu (level, time remaining). Hidden without a battery
/// (UPower's display device absent: a desktop).
#[component]
pub fn BatteryItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    let battery = use_battery()();
    if battery.presence == Presence::Absent {
        return rsx! {};
    }
    let [level, _] = battery_lines(&battery);
    rsx! {
        StatusIcon { menu: BarMenu::Battery, icon: battery_icon(&battery), tooltip: level }
    }
}
