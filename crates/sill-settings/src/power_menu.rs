//! `power_menu.*` (design/20 §1.8: the power menu, a Sheet over a Scrim). All Advanced.

use ds_core::word::Word;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

use crate::control_center::PanelMaterial;

/// Which power menu button has the keyboard when the menu opens (Enter carries it out).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
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
        label = "Power menu button",
        help = "Which button Return presses when it opens.",
        section = "Menus",
        advanced
    )]
    pub default_action: PowerMenuDefault,
    #[settings(
        label = "Power menu material",
        help = "The panel's look.",
        section = "Menus",
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
