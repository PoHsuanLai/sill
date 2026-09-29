//! `session.*`: sill's own lock screen (design/20 §1.9); design/22 §3.19 has the rows. The lock
//! grace lives here because it belongs to the lock screen, but its `KeySpec` is on the Power page
//! with `idle.*` (design/22 §3.19; `crate::schema` moves it, the derive has one page a struct).

use crate::file::ShellFile;
use crate::section::Section;
use ds_settings::Secs;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// How sill's own lock screen writes the time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, ds::Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum LockClock {
    /// `14:05`, as the bar's clock.
    #[default]
    H24,
    /// `2:05`, no AM/PM (macOS's lock screen).
    H12,
}

/// The lock screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "session", page = Page::Accounts)]
pub struct SessionSettings {
    #[settings(
        label = "Lock screen clock",
        help = "How sill's own lock screen writes the time: 24-hour, or 12-hour without AM or PM.",
        section = "Lock screen",
        advanced
    )]
    pub lock_clock: LockClock,
    #[settings(
        label = "Require the password after the display turns off",
        help = "Seconds after sill turns the displays off for idleness (and locks) in which any input opens the lock screen without the password; 0 always asks. Never after a lock you asked for (the Lock shortcut, `loginctl lock-session`, the launcher, the power menu), a lid close, a sleep or a wake from sleep.",
        section = "Lock screen",
        range = "0..=60",
        unit = "s",
        advanced
    )]
    pub lock_grace_s: Secs,
}

impl Default for SessionSettings {
    fn default() -> Self {
        SessionSettings {
            lock_clock: LockClock::H24,
            lock_grace_s: Secs(5),
        }
    }
}

impl Section for SessionSettings {
    const KEY: &'static str = "session";

    fn of(file: &ShellFile) -> &Self {
        &file.session
    }
}
