//! `session.*`: the lock screen and polkit agent of a sill session (design/20 §1.9 and §1.10):
//! borrowed programs at M7, and sill's own (`sill`) since M11. design/22 §3.19 has the M7 rows;
//! `session.lock_clock` (M11) is in `AWAITING_ROWS` with its row in sill FINDINGS "M11 freeze".
//! Idle timeouts are not sill's: cosmic-idle keeps them in COSMIC's own config (FINDINGS "M7").

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// How sill's own lock screen writes the time.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LockClock {
    /// `14:05`, as the bar's clock.
    #[default]
    H24,
    /// `2:05`, no AM/PM (macOS's lock screen).
    H12,
}

/// The session's borrowed parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "session", page = Page::Accounts)]
pub struct SessionSettings {
    #[settings(
        label = "Lock screen program",
        help = "`sill` is sill's own lock screen; `auto` picks COSMIC's lock screen (cosmic-greeter), else swaylock, else hyprlock, whichever is installed; `off` never locks; or one of those names, or a command line of your own.",
        section = "Lock screen",
        advanced
    )]
    pub locker: String,
    #[settings(
        label = "Password prompt program",
        help = "The polkit agent a sill session uses: `sill` is sill's own prompt; `auto` picks the KDE, GNOME, LXQt or MATE agent, else cosmic-osd, whichever is installed; `off` starts none; or one of those names, or a command line of your own.",
        section = "Password prompts",
        advanced
    )]
    pub polkit_agent: String,
    #[settings(
        label = "Lock screen clock",
        help = "How sill's own lock screen writes the time: 24-hour, or 12-hour without AM or PM.",
        section = "Lock screen",
        advanced
    )]
    pub lock_clock: LockClock,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for SessionSettings {
    fn default() -> Self {
        SessionSettings {
            locker: "auto".to_owned(),
            polkit_agent: "auto".to_owned(),
            lock_clock: LockClock::H24,
            extra: toml::Table::new(),
        }
    }
}
