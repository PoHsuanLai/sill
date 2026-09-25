//! The Battery module: the level and time left (the bar's `battery_lines`), then the power
//! profile's [`super::PowerProfileControl`]. Without a battery (a desktop) only the control
//! shows; without either the module is absent. Reads `use_battery` and `use_power_profile`.

use dioxus::prelude::*;
use ds::{Icon, ModulePanel, Text};
use sill_services::Presence;

use super::PowerProfileControl;
use crate::hooks::{use_battery, use_power_profile};
use crate::surfaces::bar::status::{battery_icon, battery_lines};
use crate::surfaces::control_center::modules::use_plate;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    let battery = use_battery()();
    let profiles = use_power_profile()();
    if (battery.presence, profiles.presence) == (Presence::Absent, Presence::Absent) {
        return rsx! {};
    }
    let head = match battery.presence {
        Presence::Absent => None,
        Presence::Present => {
            let [_, left] = battery_lines(&battery);
            Some(BatteryHead {
                glyph: battery_icon(&battery),
                percent: battery.level.0,
                left,
            })
        }
    };
    let (glyph, title, trailing) = match &head {
        Some(head) => (
            Some(head.glyph),
            Some(Text::from("Battery")),
            Some(rsx! { "{head.percent}%" }),
        ),
        None => (None, None, None),
    };
    rsx! {
        ModulePanel { glyph, title, trailing, plate: use_plate(),
            div { class: "sill-cc-battery",
                if let Some(head) = head {
                    span { class: "sill-cc-status", "{head.left}" }
                }
                PowerProfileControl {}
            }
        }
    }
}

/// The battery's header: its glyph and level, and the time left under it.
struct BatteryHead {
    glyph: Icon,
    percent: u16,
    left: String,
}
