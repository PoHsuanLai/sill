//! The Battery module: the level and time left (the bar's `battery_lines`), then the power
//! profile's [`super::PowerProfileControl`]. Without a battery (a desktop) only the control
//! shows; without either the module is absent. Reads `use_battery` and `use_power_profile`.

use dioxus::prelude::*;
use ds::Glyph;
use sill_services::Presence;

use super::PowerProfileControl;
use crate::hooks::{use_battery, use_power_profile};
use crate::surfaces::bar::status::{battery_icon, battery_lines};
use crate::surfaces::control_center::module_box::ModuleBox;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    let battery = use_battery()();
    let profiles = use_power_profile()();
    if (battery.presence, profiles.presence) == (Presence::Absent, Presence::Absent) {
        return rsx! {};
    }
    let level = match battery.presence {
        Presence::Absent => None,
        Presence::Present => {
            let [_, left] = battery_lines(&battery);
            Some(rsx! {
                div { class: "sill-cc-level-head",
                    span { class: "sill-cc-glyph", Glyph { icon: battery_icon(&battery) } }
                    span { class: "sill-cc-title", "Battery" }
                    span { class: "sill-cc-value", "{battery.level.0}%" }
                }
                span { class: "sill-cc-status", "{left}" }
            })
        }
    };
    rsx! {
        ModuleBox {
            div { class: "sill-cc-battery",
                {level}
                PowerProfileControl {}
            }
        }
    }
}
