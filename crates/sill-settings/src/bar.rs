//! `bar.*` (design/22 §3.4; design/13 §13.3.1). `bar.autohide` is Basic, on the Appearance page
//! beside the menu bar's other keys (the Mac's Control Centre pane has no page here); the rest
//! are Advanced (§5).

use ds_core::word::Word;
use ds_settings::Percent;
use ds_settings::schema::Page;
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
        section = "Menu bar",
        note = "In full screen the bar returns when the pointer rests at the top."
    )]
    pub autohide: MenuBarHiding,
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
            battery_low_percent: Percent(20),
        }
    }
}
