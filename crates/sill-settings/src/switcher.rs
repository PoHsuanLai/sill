//! `switcher.*` (design/22 §3.11; design/13 §13.3.5): the app switcher. All Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

/// The app switcher's timing and cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "switcher", page = Page::KeyboardAndShortcuts)]
pub struct SwitcherSettings {
    #[settings(
        label = "Show delay",
        help = "How long the chord is held before the switcher appears.",
        section = "App switcher",
        range = "0..=500",
        unit = "ms",
        advanced
    )]
    pub show_delay_ms: Ms,
    #[settings(
        label = "Icon size",
        help = "The size of an app icon in the switcher.",
        section = "App switcher",
        range = "32..=256",
        unit = "px",
        advanced
    )]
    pub icon_size_px: Px,
    #[settings(
        label = "Cell size",
        help = "The size of one app's cell.",
        section = "App switcher",
        range = "48..=256",
        unit = "px",
        advanced
    )]
    pub cell_size_px: Px,
    #[settings(
        label = "Cell gap",
        help = "Space between cells.",
        section = "App switcher",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub cell_gap_px: Px,
    #[settings(
        label = "Smallest icon",
        help = "How small icons may shrink with many apps open.",
        section = "App switcher",
        range = "16..=128",
        unit = "px",
        advanced
    )]
    pub overflow_min_icon_px: Px,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for SwitcherSettings {
    fn default() -> Self {
        SwitcherSettings {
            show_delay_ms: Ms(150),
            icon_size_px: Px(96),
            cell_size_px: Px(112),
            cell_gap_px: Px(8),
            overflow_min_icon_px: Px(48),
            extra: toml::Table::new(),
        }
    }
}
