//! `power_menu.*` (design/20 §1.8: the power menu, a Sheet over a Scrim). No design/22 rows yet;
//! both keys are in `tests/keys.rs` `AWAITING_ROWS` and FINDINGS "M5 freeze" has their row text.
//! All Advanced.

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

use crate::control_center::PanelMaterial;

/// Which power menu button has the keyboard when the menu opens (Enter carries it out).
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
pub enum PowerMenuDefault {
    LogOut,
    Restart,
    /// macOS's shut-down dialog focuses Shut Down.
    #[default]
    ShutDown,
    Suspend,
}

/// The power menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "power_menu", page = Page::Appearance)]
pub struct PowerMenuSettings {
    #[settings(
        label = "Focused button",
        help = "Which power menu button Return presses when the menu opens.",
        section = "Power menu",
        advanced
    )]
    pub default_action: PowerMenuDefault,
    #[settings(
        label = "Material",
        help = "The power menu panel's material.",
        section = "Power menu",
        advanced
    )]
    pub material: PanelMaterial,
}

impl Default for PowerMenuSettings {
    fn default() -> Self {
        PowerMenuSettings {
            default_action: PowerMenuDefault::ShutDown,
            material: PanelMaterial::Sheet,
        }
    }
}
