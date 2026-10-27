//! `bar.*` (design/22 §3.4; design/13 §13.3.1). `bar.autohide` is Basic, on the Appearance page
//! beside the menu bar's other keys (the Mac's Control Centre pane has no page here); the rest
//! are Advanced (§5).

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Percent, Px};
use serde::{Deserialize, Serialize};

/// `bar.autohide`: when the menu bar hides and shows (macOS: System Settings, Control Centre,
/// "Automatically hide and show the menu bar"). `FullScreenOnly` is macOS's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum MenuBarHiding {
    /// Hidden on the desktop and in full screen; a pointer at the top edge brings it.
    Always,
    /// Hidden on the desktop; in full screen it stays.
    #[word(label = "On Desktop Only")]
    DesktopOnly,
    /// Fixed on the desktop; hidden in full screen until the pointer rests at the top edge.
    #[default]
    #[word(label = "In Full Screen Only")]
    FullScreenOnly,
    /// Always fixed, in full screen too.
    Never,
}

/// The top bar's geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "bar", page = Page::Appearance)]
pub struct BarSettings {
    /// `bar.autohide`.
    #[settings(
        label = "Automatically hide and show the menu bar",
        help = "Shows the menu bar when the pointer reaches the top.",
        section = "Menu bar"
    )]
    pub autohide: MenuBarHiding,
    /// `bar.height_px` (settled 24, design/29 §13 decision 2; quire's `SHELL_SCALE.bar`).
    #[settings(
        label = "Bar height",
        help = "How tall the menu bar is.",
        section = "Menu bar",
        range = "20..=64",
        unit = "px",
        advanced
    )]
    pub height_px: Px,
    /// `bar.status_icon_box_px`.
    #[settings(
        label = "Status item box",
        help = "The box each status icon sits in.",
        section = "Menu bar",
        range = "16..=32",
        unit = "px",
        advanced
    )]
    pub status_icon_box_px: Px,
    /// `bar.status_glyph_px`.
    #[settings(
        label = "Status glyph",
        help = "How big status icons are.",
        section = "Menu bar",
        range = "12..=32",
        unit = "px",
        advanced
    )]
    pub status_glyph_px: Px,
    /// `bar.battery_low_percent` (quire design/26 G10): at or under this charge, on
    /// battery, the bar's battery fill turns `--battery-low`.
    #[settings(
        label = "Low battery",
        help = "The battery icon turns red at this charge or lower.",
        section = "Menu bar",
        range = "5..=50",
        unit = "%",
        advanced
    )]
    pub battery_low_percent: Percent,
}

impl Default for BarSettings {
    fn default() -> Self {
        BarSettings {
            autohide: MenuBarHiding::FullScreenOnly,
            height_px: Px(24),
            status_icon_box_px: Px(22),
            status_glyph_px: Px(16),
            battery_low_percent: Percent(20),
        }
    }
}
