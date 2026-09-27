//! The Battery module: the bar item's battery glyph, the level and time left (the bar's
//! `battery_lines`), then the power profile's [`super::PowerProfileControl`]. Without a battery
//! (a desktop) only the control shows; without either the module is absent. Reads
//! `use_battery`, `use_power_profile` and `bar.battery_low_percent`.

use dioxus::prelude::*;
use ds::detail::FirstShow;
use ds::{BatteryState, IconSource, ModulePanel, StatusState, Text};
use sill_services::Presence;

use super::PowerProfileControl;
use crate::hooks::{use_battery, use_power_profile, use_settings};
use crate::surfaces::bar::glyph_state::battery_glyph;
use crate::surfaces::bar::status::battery_lines;
use crate::surfaces::control_center::modules::use_plate;

/// The Battery module.
#[component]
pub fn BatteryModule() -> Element {
    let battery = use_battery()();
    let profiles = use_power_profile()();
    let low = use_settings()().bar.battery_low_percent;
    if (battery.presence, profiles.presence) == (Presence::Absent, Presence::Absent) {
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
        // The center was just opened: the fill sweeps in from empty (quire Q391).
        ModulePanel { glyph, first: FirstShow::Animate, title, trailing, plate: use_plate(),
            div { class: "sill-cc-battery",
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
