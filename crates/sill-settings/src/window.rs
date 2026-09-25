//! `window.*`: how the shell's own app windows handle a titlebar drag and the green light's
//! tile menu (quire CONSUMING "Window frame"). The rows are design/22's `window` section;
//! `tests/keys.rs` lists them in `AWAITING_ROWS` until it lands. All Advanced.

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
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
    #[settings(
        label = "Hold to show the tile menu",
        help = "How long a press on the green light waits before it shows the tile menu instead of zooming.",
        section = "Windows",
        range = "100..=3000",
        unit = "ms",
        advanced
    )]
    pub tile_menu_press_ms: Ms,
    #[settings(
        label = "Hover to show the tile menu",
        help = "How long the pointer rests on the green light before the tile menu shows.",
        section = "Windows",
        range = "100..=5000",
        unit = "ms",
        advanced
    )]
    pub tile_menu_hover_ms: Ms,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for WindowSettings {
    fn default() -> Self {
        WindowSettings {
            move_threshold_px: Px(4),
            tile_menu_press_ms: Ms(500),
            tile_menu_hover_ms: Ms(800),
            extra: toml::Table::new(),
        }
    }
}
