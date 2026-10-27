//! `idle.*`: sill's own idle service: the dim, the screen off, the lock at screen off and the idle
//! suspend. design/22 §3.24 has the rows. Seconds are
//! `Secs` and minutes `Mins`, plain integers in the file as before; every key is on the Power page,
//! Advanced, with `session.lock_grace_s` (`crate::schema` moves that one key).

use ds_settings::schema::Page;
use ds_settings::{Mins, Percent, Secs};
use serde::{Deserialize, Serialize};
use sill_model::idle::IdleLock;

/// sill's idle service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "idle", page = Page::Power)]
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
        label = "Dim before the display turns off",
        help = "Seconds of dimming as a warning; 0 skips it.",
        section = "Display & sleep",
        range = "0..=60",
        unit = "s",
        advanced
    )]
    pub dim_s: Secs,
    #[settings(
        label = "Dimmed brightness",
        help = "How bright the display stays while dimmed.",
        section = "Display & sleep",
        range = "10..=90",
        unit = "%",
        advanced
    )]
    pub dim_level_pct: Percent,
    #[settings(
        label = "Lock when the display turns off",
        help = "Locks when idle turns the display off; sleep always locks.",
        section = "Display & sleep",
        advanced
    )]
    pub lock: IdleLock,
    #[settings(
        label = "Turn the display off on the lock screen after",
        help = "Seconds without input; 0 uses the normal time.",
        section = "Display & sleep",
        range = "0..=600",
        unit = "s",
        advanced
    )]
    pub locked_screen_off_s: Secs,
}

impl Default for IdleSettings {
    fn default() -> Self {
        IdleSettings {
            screen_off_ac_min: Mins(15),
            screen_off_battery_min: Mins(15),
            suspend_ac_min: Mins(30),
            suspend_battery_min: Mins(15),
            dim_s: Secs(5),
            dim_level_pct: Percent(50),
            lock: IdleLock::AtScreenOff,
            locked_screen_off_s: Secs(60),
        }
    }
}
