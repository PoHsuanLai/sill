//! `bar.*` (design/22 §3.4; design/13 §13.3.1). All Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Count, Px};
use serde::{Deserialize, Serialize};

/// Which box a bar status glyph is drawn at (design/13 §13.9 open decision 8).
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
pub enum BarGlyphSize {
    /// A 16 px glyph in a 22 px box (macOS status icons).
    #[default]
    StatusIcon16,
    /// The glyph fills quire's `IconSize::Bar` 22 px box.
    IconSizeBar22,
}

/// The top bar's geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "bar", page = Page::Appearance)]
pub struct BarSettings {
    /// `bar.height_px` (proposed 32; flagged: macOS 24).
    #[settings(
        label = "Bar height",
        help = "The top bar's height; windows are placed below it.",
        section = "Bar",
        range = "20..=64",
        unit = "px",
        advanced
    )]
    pub height_px: Px,
    /// `bar.title_hit_height_px`.
    #[settings(
        label = "Title hit height",
        help = "The clickable height of a bar title or status item.",
        section = "Bar",
        range = "16..=48",
        unit = "px",
        advanced
    )]
    pub title_hit_height_px: Px,
    /// `bar.title_padding_px`.
    #[settings(
        label = "Title padding",
        help = "Horizontal padding inside a bar title.",
        section = "Bar",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub title_padding_px: Px,
    /// `bar.open_title_pill_height_px`.
    #[settings(
        label = "Open title highlight",
        help = "The height of the pill behind a title whose menu is open.",
        section = "Bar",
        range = "16..=48",
        unit = "px",
        advanced
    )]
    pub open_title_pill_height_px: Px,
    /// `bar.status_icon_box_px`.
    #[settings(
        label = "Status item box",
        help = "The box a status item's glyph sits in.",
        section = "Bar",
        range = "16..=32",
        unit = "px",
        advanced
    )]
    pub status_icon_box_px: Px,
    /// `bar.status_glyph_px`.
    #[settings(
        label = "Status glyph",
        help = "The size of a status item's glyph.",
        section = "Bar",
        range = "12..=32",
        unit = "px",
        advanced
    )]
    pub status_glyph_px: Px,
    /// `bar.status_gap_px`.
    #[settings(
        label = "Status gap",
        help = "Space between status items.",
        section = "Bar",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub status_gap_px: Px,
    /// `bar.glyph_size_policy`.
    #[settings(
        label = "Status glyph size",
        help = "A 16 px glyph in its box, or a glyph that fills the 22 px box.",
        section = "Bar",
        advanced
    )]
    pub glyph_size_policy: BarGlyphSize,
    /// `bar.item_font_px` (the macOS polish pass: menu bar text 13).
    #[settings(
        label = "Item text size",
        help = "The size of a bar title's and the clock's text.",
        section = "Bar",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub item_font_px: Px,
    /// `bar.item_font_weight`.
    #[settings(
        label = "Item text weight",
        help = "The weight of a bar title's and the clock's text; the app name is always bold.",
        section = "Bar",
        range = "100..=900",
        advanced
    )]
    pub item_font_weight: Count,
    /// `bar.item_radius_px`.
    #[settings(
        label = "Item highlight radius",
        help = "The corner radius of the highlight behind a bar item.",
        section = "Bar",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub item_radius_px: Px,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for BarSettings {
    fn default() -> Self {
        BarSettings {
            height_px: Px(32),
            title_hit_height_px: Px(24),
            title_padding_px: Px(10),
            open_title_pill_height_px: Px(24),
            status_icon_box_px: Px(22),
            status_glyph_px: Px(16),
            status_gap_px: Px(4),
            glyph_size_policy: BarGlyphSize::StatusIcon16,
            item_font_px: Px(13),
            item_font_weight: Count(500),
            item_radius_px: Px(4),
            extra: toml::Table::new(),
        }
    }
}
