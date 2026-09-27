//! `session.*`: the lock screen and polkit agent of a sill session (design/20 §1.9 and §1.10):
//! borrowed programs at M7, and sill's own (`sill`) since M11. design/22 §3.19 has the rows.
//! `session.locker` defaults to `sill`, sill's own lock screen (the user's pick, 2026-09-27;
//! the changed row text is in sill FINDINGS "User picks: lock and OSD", F861).
//! Idle is sill's own since Q420 B (`idle.*`, FINDINGS "sill idle (Q420 B)"); the lock grace
//! lives here because it belongs to the lock screen, but its `KeySpec` is on the Power page with
//! `idle.*` (design/22 §3.19, Q445; `crate::schema` moves it, the derive has one page a struct).

use ds_settings::Secs;
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
        help = "`sill` (the default) is sill's own lock screen; `auto` picks COSMIC's lock screen (cosmic-greeter), else swaylock, else hyprlock, whichever is installed; `off` never locks; or one of those names, or a command line of your own.",
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
    #[settings(
        label = "Your picture",
        help = "The picture on the lock screen: `auto` is your photo (`~/.face`) if you have one, else your initial; `letter`; `photo`; or `emoji:<name>` for one of the design system's reacting emoji.",
        section = "Lock screen",
        advanced
    )]
    pub user_picture: String,
    #[settings(
        label = "Require the password after the display turns off",
        help = "Seconds after sill turns the displays off for idleness (and locks) in which any input opens the lock screen without the password; 0 always asks. Never after a lock you asked for (the Lock shortcut, `loginctl lock-session`, the launcher, the power menu), a lid close, a sleep or a wake from sleep. Only with `session.locker = \"sill\"`.",
        section = "Lock screen",
        range = "0..=60",
        unit = "s",
        advanced
    )]
    pub lock_grace_s: Secs,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for SessionSettings {
    fn default() -> Self {
        SessionSettings {
            locker: "sill".to_owned(),
            polkit_agent: "auto".to_owned(),
            lock_clock: LockClock::H24,
            user_picture: "auto".to_owned(),
            lock_grace_s: Secs(5),
            extra: toml::Table::new(),
        }
    }
}
