//! `window.*`: how the shell's own app windows handle a titlebar drag. The rows are design/22's
//! `window` section. All Advanced.

use crate::file::ShellFile;
use crate::section::Section;
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
        help = "How far a press on a titlebar must travel before the window starts to move.",
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

impl Section for WindowSettings {
    const KEY: &'static str = "window";

    fn of(file: &ShellFile) -> &Self {
        &file.window
    }
}
