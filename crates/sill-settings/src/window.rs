//! `window.*`: how the shell's own app windows handle a titlebar drag. The rows are design/22's
//! `window` section. All Advanced.

use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// The shell's app windows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "window", page = Page::Appearance)]
pub struct WindowSettings {
    #[settings(
        label = "Drag distance before a window moves",
        help = "How far to drag a titlebar before the window moves.",
        section = "Windows",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub move_threshold_px: Px,
}

impl Default for WindowSettings {
    fn default() -> Self {
        WindowSettings {
            move_threshold_px: Px(4),
        }
    }
}
