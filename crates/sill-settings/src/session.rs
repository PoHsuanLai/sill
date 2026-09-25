//! `session.*`: the borrowed lock screen and polkit agent of a sill session (design/20 §1.9 and
//! §1.10, M7). design/22 has no rows for them yet; `tests/keys.rs` lists them in
//! `AWAITING_ROWS` and sill FINDINGS "M7" has the proposed row text. Idle timeouts are not
//! sill's: cosmic-idle keeps them in COSMIC's own config (FINDINGS "M7").

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// The session's borrowed parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "session", page = Page::Accounts)]
pub struct SessionSettings {
    #[settings(
        label = "Lock screen program",
        help = "`auto` picks COSMIC's lock screen (cosmic-greeter), else swaylock, else hyprlock, whichever is installed; `off` never locks; or one of those names, or a command line of your own.",
        section = "Lock screen",
        advanced
    )]
    pub locker: String,
    #[settings(
        label = "Password prompt program",
        help = "The polkit agent a sill session starts: `auto` picks the KDE, GNOME, LXQt or MATE agent, else cosmic-osd, whichever is installed; `off` starts none; or one of those names, or a command line of your own.",
        section = "Password prompts",
        advanced
    )]
    pub polkit_agent: String,
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
            extra: toml::Table::new(),
        }
    }
}
