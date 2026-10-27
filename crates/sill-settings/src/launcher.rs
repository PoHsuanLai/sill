//! `launcher.*` (design/22 §3.6 and `launcher.material` from §3.1; design/13 §13.3.9). All
//! Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Count, Fraction, Ms, Px};
use serde::{Deserialize, Serialize};

use crate::control_center::PanelMaterial;
use crate::launcher_kinds::WebSearch;
use sill_launcher::SkinTone;
use sill_model::clipboard::HistoryMode;
use sill_model::results::{CurrencyPolicy, FilesChoice};

/// The launcher panel's placement and rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "launcher", page = Page::Appearance)]
pub struct LauncherSettings {
    /// Panel top = `max(bar_h + top_bar_gap, round(top_vertical_fraction x H - top_centre_bias))`.
    #[settings(
        label = "Gap below the bar",
        help = "The least space between the bar and the launcher.",
        section = "Position",
        range = "0..=200",
        unit = "px",
        advanced
    )]
    pub top_bar_gap_px: Px,
    #[settings(
        label = "Height bias",
        help = "Moves the launcher up by this much.",
        section = "Position",
        range = "0..=1000",
        unit = "px",
        advanced
    )]
    pub top_centre_bias_px: Px,
    #[settings(
        label = "Vertical position",
        help = "Where the launcher sits, as a share of the screen's height.",
        section = "Position",
        range = "0..=1000",
        advanced
    )]
    pub top_vertical_fraction: Fraction,
    /// `launcher.material` (§3.1 row, owned by sill).
    #[settings(
        label = "Panel material",
        help = "The launcher panel's look.",
        section = "Position",
        advanced
    )]
    pub material: PanelMaterial,
    /// `launcher.field_font_px` (the macOS polish pass: Spotlight's 22 pt query).
    #[settings(
        label = "Search text size",
        help = "The size of what you type.",
        section = "Text",
        range = "12..=40",
        unit = "px",
        advanced
    )]
    pub field_font_px: Px,
    /// `launcher.field_font_weight`.
    #[settings(
        label = "Search text weight",
        help = "The weight of what you type.",
        section = "Text",
        range = "100..=900",
        advanced
    )]
    pub field_font_weight: Count,
    /// `launcher.field_glyph_px`.
    #[settings(
        label = "Search glyph size",
        help = "The size of the magnifier icon.",
        section = "Text",
        range = "12..=40",
        unit = "px",
        advanced
    )]
    pub field_glyph_px: Px,
    /// `launcher.row_title_px`.
    #[settings(
        label = "Result title size",
        help = "The size of a result's title.",
        section = "Text",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub row_title_px: Px,
    /// `launcher.row_detail_px`.
    #[settings(
        label = "Result detail size",
        help = "The size of a result's detail line.",
        section = "Text",
        range = "9..=20",
        unit = "px",
        advanced
    )]
    pub row_detail_px: Px,
    /// `launcher.files_backend`.
    #[settings(
        label = "File search",
        help = "How the launcher finds documents; off skips files.",
        section = "Search",
        advanced
    )]
    pub files_backend: FilesChoice,
    /// `launcher.files_budget_ms`.
    #[settings(
        label = "File search time limit",
        help = "How long a search runs before showing what it found.",
        section = "Search",
        range = "20..=2000",
        unit = "ms",
        advanced
    )]
    pub files_budget_ms: Ms,
    /// `launcher.clipboard_history`.
    #[settings(
        label = "Clipboard history",
        help = "Keeps copies in memory only; password copies are never kept.",
        section = "Clipboard",
        advanced
    )]
    pub clipboard_history: HistoryMode,
    /// `launcher.clipboard_cap`.
    #[settings(
        label = "Clipboard history length",
        help = "How many copies it keeps; the oldest go first.",
        section = "Clipboard",
        range = "1..=500",
        advanced
    )]
    pub clipboard_cap: Count,
    /// `launcher.clipboard_image_max_mb`.
    #[settings(
        label = "Largest copied image kept",
        help = "Bigger images aren't kept; 0 keeps none.",
        section = "Clipboard",
        range = "0..=64",
        unit = "MB",
        advanced
    )]
    pub clipboard_image_max_mb: Count,
    /// `launcher.emoji_skin_tone`.
    #[settings(
        label = "Emoji skin tone",
        help = "Used for emoji that have one.",
        section = "Search",
        advanced
    )]
    pub emoji_skin_tone: SkinTone,
    /// `launcher.web_search`.
    #[settings(
        label = "Search the web",
        help = "Offers a web search as the last result.",
        section = "Search",
        advanced
    )]
    pub web_search: WebSearch,
    /// `launcher.web_engine_url`.
    #[settings(
        label = "Web search address",
        help = "Use {query} where your words go; web addresses only.",
        section = "Search",
        advanced
    )]
    pub web_engine_url: String,
    /// `launcher.currency`.
    #[settings(
        label = "Currency conversion",
        help = "Converts currencies with daily rates; off fetches nothing.",
        section = "Search",
        advanced
    )]
    pub currency: CurrencyPolicy,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        LauncherSettings {
            top_bar_gap_px: Px(24),
            top_centre_bias_px: Px(230),
            top_vertical_fraction: Fraction(400),
            material: PanelMaterial::Sheet,
            field_font_px: Px(22),
            field_font_weight: Count(500),
            field_glyph_px: Px(20),
            row_title_px: Px(14),
            row_detail_px: Px(12),
            files_backend: FilesChoice::Auto,
            files_budget_ms: Ms(150),
            clipboard_history: HistoryMode::Memory,
            clipboard_cap: Count(50),
            clipboard_image_max_mb: Count(8),
            emoji_skin_tone: SkinTone::Default,
            web_search: WebSearch::On,
            web_engine_url: "https://duckduckgo.com/?q={query}".to_owned(),
            currency: CurrencyPolicy::Off,
        }
    }
}
