//! `bar.*` (design/22 §3.4; design/13 §13.3.1). `bar.autohide` is Basic, on the Appearance page
//! beside the menu bar's other keys (the Mac's Control Centre pane has no page here); the rest
//! are Advanced (§5).

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Count, Percent, Px};
use serde::{Deserialize, Serialize};

/// Which box a bar status glyph is drawn at (design/13 §13.9 open decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum BarGlyphSize {
    /// A 16 px glyph in a 22 px box (macOS status icons).
    #[default]
    StatusIcon16,
    /// The glyph fills quire's `IconSize::Bar` 22 px box.
    IconSizeBar22,
}

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
    /// `bar.title_padding_px`.
    #[settings(
        label = "Title padding",
        help = "Space around each menu title.",
        section = "Menu bar",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub title_padding_px: Px,
    /// `bar.open_title_pill_height_px`.
    #[settings(
        label = "Open title highlight",
        help = "How tall the highlight is behind an open menu.",
        section = "Menu bar",
        range = "16..=48",
        unit = "px",
        advanced
    )]
    pub open_title_pill_height_px: Px,
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
    /// `bar.status_gap_px`.
    #[settings(
        label = "Status gap",
        help = "Space between status items.",
        section = "Menu bar",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub status_gap_px: Px,
    /// `bar.glyph_size_policy`.
    #[settings(
        label = "Status glyph size",
        help = "A small glyph in its box, or one that fills the box.",
        section = "Menu bar",
        advanced
    )]
    pub glyph_size_policy: BarGlyphSize,
    /// `bar.item_font_px` (the macOS polish pass: menu bar text 13).
    #[settings(
        label = "Item text size",
        help = "Text size of menu titles and the clock.",
        section = "Menu bar",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub item_font_px: Px,
    /// `bar.item_font_weight`.
    #[settings(
        label = "Item text weight",
        help = "Weight of titles and the clock; app names stay bold.",
        section = "Menu bar",
        range = "100..=900",
        advanced
    )]
    pub item_font_weight: Count,
    /// `bar.item_radius_px`.
    #[settings(
        label = "Item highlight radius",
        help = "How round the highlight behind an item is.",
        section = "Menu bar",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub item_radius_px: Px,
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
            title_padding_px: Px(8),
            open_title_pill_height_px: Px(22),
            status_icon_box_px: Px(22),
            status_glyph_px: Px(16),
            status_gap_px: Px(0),
            glyph_size_policy: BarGlyphSize::StatusIcon16,
            item_font_px: Px(13),
            item_font_weight: Count(500),
            item_radius_px: Px(5),
            battery_low_percent: Percent(20),
        }
    }
}
