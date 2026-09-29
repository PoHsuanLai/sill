//! `idle.*`: sill's own idle service: the dim, the screen off, the lock at screen off and the idle
//! suspend that cosmic-idle did before. The screen-off and suspend times come from COSMIC's own
//! config (`com.system76.CosmicIdle`, which COSMIC Settings' Power page edits) unless `idle.times`
//! is `sill`; the rest has no COSMIC key and is sill's. design/22 §3.24 has the rows. Seconds are
//! `Secs` and minutes `Mins`, plain integers in the file as before; every key is on the Power page,
//! Advanced, with `session.lock_grace_s` (`crate::schema` moves that one key).

use crate::file::ShellFile;
use crate::section::Section;
use ds_settings::schema::Page;
use ds_settings::{Mins, Percent, Secs};
use serde::{Deserialize, Serialize};

/// `idle.times`: where the screen-off and suspend times come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ds::Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum IdleTimesFrom {
    /// COSMIC's `com.system76.CosmicIdle` (COSMIC Settings' Power page): one screen-off time for
    /// both power sources, and suspend on AC and on battery.
    #[default]
    Cosmic,
    /// The four `idle.*_min` keys below.
    Sill,
}

/// `idle.lock`: whether turning the screens off for idleness locks the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ds::Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum IdleLockAt {
    /// Lock as the screens go off (what cosmic-idle did; macOS's "require password").
    #[default]
    ScreenOff,
    /// Never lock for idleness; a suspend still locks.
    Never,
}

/// sill's idle service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "idle", page = Page::Power)]
pub struct IdleSettings {
    #[settings(
        label = "Idle times from",
        help = "`cosmic` uses the times COSMIC Settings' Power page sets (one screen-off time, suspend on AC and on battery); `sill` uses the four times below instead.",
        section = "Idle",
        advanced
    )]
    pub times: IdleTimesFrom,
    #[settings(
        label = "Turn the display off on power after",
        help = "Minutes without input before the displays go off while on AC power; 0 never. Only with `idle.times = \"sill\"`.",
        section = "Idle",
        range = "0..=240",
        unit = "min",
        advanced
    )]
    pub screen_off_ac_min: Mins,
    #[settings(
        label = "Turn the display off on battery after",
        help = "Minutes without input before the displays go off on battery; 0 never. Only with `idle.times = \"sill\"`.",
        section = "Idle",
        range = "0..=240",
        unit = "min",
        advanced
    )]
    pub screen_off_battery_min: Mins,
    #[settings(
        label = "Sleep on power after",
        help = "Minutes without input before the computer sleeps while on AC power; 0 never. Only with `idle.times = \"sill\"`.",
        section = "Idle",
        range = "0..=480",
        unit = "min",
        advanced
    )]
    pub suspend_ac_min: Mins,
    #[settings(
        label = "Sleep on battery after",
        help = "Minutes without input before the computer sleeps on battery; 0 never. Only with `idle.times = \"sill\"`.",
        section = "Idle",
        range = "0..=480",
        unit = "min",
        advanced
    )]
    pub suspend_battery_min: Mins,
    #[settings(
        label = "Dim before the display turns off",
        help = "Seconds the displays spend dimmed before they turn off, a warning any input cancels; 0 turns them straight off.",
        section = "Idle",
        range = "0..=60",
        unit = "s",
        advanced
    )]
    pub dim_s: Secs,
    #[settings(
        label = "Dimmed brightness",
        help = "How bright the displays stay while dimmed, as a share of their brightness.",
        section = "Idle",
        range = "10..=90",
        unit = "%",
        advanced
    )]
    pub dim_level_pct: Percent,
    #[settings(
        label = "Lock when the display turns off",
        help = "`screen_off` locks as the displays go off for idleness (`session.lock_grace_s` still lets a quick wake in); `never` does not (a sleep still locks).",
        section = "Idle",
        advanced
    )]
    pub lock: IdleLockAt,
    #[settings(
        label = "Turn the display off on the lock screen after",
        help = "Seconds without input on the lock screen before the displays go off again; 0 uses the normal screen-off time.",
        section = "Idle",
        range = "0..=600",
        unit = "s",
        advanced
    )]
    pub locked_screen_off_s: Secs,
}

impl Default for IdleSettings {
    fn default() -> Self {
        IdleSettings {
            times: IdleTimesFrom::Cosmic,
            // COSMIC's own defaults (cosmic-idle-config at epoch-1.8.0), so switching
            // `idle.times` to `sill` changes nothing until a time is edited.
            screen_off_ac_min: Mins(15),
            screen_off_battery_min: Mins(15),
            suspend_ac_min: Mins(30),
            suspend_battery_min: Mins(15),
            dim_s: Secs(5),
            dim_level_pct: Percent(50),
            lock: IdleLockAt::ScreenOff,
            locked_screen_off_s: Secs(60),
        }
    }
}

impl Section for IdleSettings {
    const KEY: &'static str = "idle";

    fn of(file: &ShellFile) -> &Self {
        &file.idle
    }
}
