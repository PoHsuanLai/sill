//! `idle.*`: sill's own idle service: the screen off, the lock at screen off and the idle suspend
//! (the dim is constants in the daemon's policy). design/22 §3.24 has the rows. Minutes are
//! `Mins`, plain integers in the file; every key is on the Power page, Advanced, with
//! `session.lock_grace_s` (`crate::schema` moves that one key).

use ds_settings::Mins;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::idle::IdleLock;

/// sill's idle service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(
    file = "sill/settings.toml", domain = "idle", page = Page::Power,
    footnote(section = "Display & sleep", text = "Dimming warns you before the display turns off. Any input cancels it."),
)]
pub struct IdleSettings {
    #[settings(
        label = "Turn the display off on power after",
        help = "Minutes without input; 0 means never.",
        section = "Display & sleep",
        range = "0..=240",
        unit = "min",
        advanced
    )]
    pub screen_off_ac_min: Mins,
    #[settings(
        label = "Turn the display off on battery after",
        help = "Minutes without input; 0 means never.",
        section = "Display & sleep",
        range = "0..=240",
        unit = "min",
        advanced
    )]
    pub screen_off_battery_min: Mins,
    #[settings(
        label = "Sleep on power after",
        help = "Minutes without input; 0 means never.",
        section = "Display & sleep",
        range = "0..=480",
        unit = "min",
        advanced
    )]
    pub suspend_ac_min: Mins,
    #[settings(
        label = "Sleep on battery after",
        help = "Minutes without input; 0 means never.",
        section = "Display & sleep",
        range = "0..=480",
        unit = "min",
        advanced
    )]
    pub suspend_battery_min: Mins,
    #[settings(
        label = "Lock when the display turns off",
        help = "Locks when idle turns the display off; sleep always locks.",
        section = "Display & sleep",
        advanced,
        note = "Waking quickly can still skip the password (see Lock screen)."
    )]
    pub lock: IdleLock,
}

impl Default for IdleSettings {
    fn default() -> Self {
        IdleSettings {
            screen_off_ac_min: Mins(15),
            screen_off_battery_min: Mins(15),
            suspend_ac_min: Mins(30),
            suspend_battery_min: Mins(15),
            lock: IdleLock::AtScreenOff,
        }
    }
}
