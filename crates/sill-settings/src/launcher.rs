//! `launcher.*` (design/22 §3.6 and `launcher.material` from §3.1; design/13 §13.3.9). All
//! Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Count, Fraction, Ms, Px};
use serde::{Deserialize, Serialize};

use crate::control_center::PanelMaterial;
use crate::launcher_kinds::{
    ClipboardHistory, CurrencyRates, EmojiSkinTone, FilesBackend, WebSearch,
};

/// The launcher panel's placement and rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "launcher", page = Page::Appearance)]
pub struct LauncherSettings {
    /// Panel top = `max(bar_h + top_bar_gap, round(top_vertical_fraction x H - top_centre_bias))`.
    #[settings(
        label = "Gap below the bar",
        help = "The least space between the bar and the launcher.",
        section = "Launcher",
        range = "0..=200",
        unit = "px",
        advanced
    )]
    pub top_bar_gap_px: Px,
    #[settings(
        label = "Height bias",
        help = "Subtracted from the launcher's vertical position.",
        section = "Launcher",
        range = "0..=1000",
        unit = "px",
        advanced
    )]
    pub top_centre_bias_px: Px,
    #[settings(
        label = "Vertical position",
        help = "Where the launcher sits, as a fraction of the screen's height (stored per-mille).",
        section = "Launcher",
        range = "0..=1000",
        advanced
    )]
    pub top_vertical_fraction: Fraction,
    #[settings(
        label = "Search row height",
        help = "The height of the search field row.",
        section = "Launcher",
        range = "32..=96",
        unit = "px",
        advanced
    )]
    pub input_row_height_px: Px,
    #[settings(
        label = "Result row height",
        help = "The height of one result row.",
        section = "Launcher",
        range = "24..=96",
        unit = "px",
        advanced
    )]
    pub result_row_height_px: Px,
    #[settings(
        label = "Open latency budget",
        help = "The p95 open time the acceptance script holds the launcher to.",
        section = "Launcher",
        range = "10..=1000",
        unit = "ms",
        advanced
    )]
    pub open_latency_budget_ms: Ms,
    /// `launcher.material` (§3.1 row, owned by sill: quire FINDINGS F15).
    #[settings(
        label = "Panel material",
        help = "The launcher panel's material.",
        section = "Launcher",
        advanced
    )]
    pub material: PanelMaterial,
    /// `launcher.field_font_px` (the macOS polish pass: Spotlight's 22 pt query).
    #[settings(
        label = "Search text size",
        help = "The size of the query in the search field.",
        section = "Launcher",
        range = "12..=40",
        unit = "px",
        advanced
    )]
    pub field_font_px: Px,
    /// `launcher.field_font_weight`.
    #[settings(
        label = "Search text weight",
        help = "The weight of the query in the search field.",
        section = "Launcher",
        range = "100..=900",
        advanced
    )]
    pub field_font_weight: Count,
    /// `launcher.field_glyph_px`.
    #[settings(
        label = "Search glyph size",
        help = "The size of the magnifier beside the query.",
        section = "Launcher",
        range = "12..=40",
        unit = "px",
        advanced
    )]
    pub field_glyph_px: Px,
    /// `launcher.row_title_px`.
    #[settings(
        label = "Result title size",
        help = "The size of a result's title.",
        section = "Launcher",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub row_title_px: Px,
    /// `launcher.row_detail_px`.
    #[settings(
        label = "Result detail size",
        help = "The size of a result's detail line.",
        section = "Launcher",
        range = "9..=20",
        unit = "px",
        advanced
    )]
    pub row_detail_px: Px,
    /// `launcher.files_backend` (M9).
    #[settings(
        label = "File search",
        help = "Where the launcher finds documents: `auto` asks Tracker, else searches your home folder with fd; or name one; `off` searches no files.",
        section = "Launcher",
        advanced
    )]
    pub files_backend: FilesBackend,
    /// `launcher.files_budget_ms` (M9).
    #[settings(
        label = "File search time limit",
        help = "How long one file search may take before the launcher shows what it found so far.",
        section = "Launcher",
        range = "20..=2000",
        unit = "ms",
        advanced
    )]
    pub files_budget_ms: Ms,
    /// `launcher.clipboard_history` (M9).
    #[settings(
        label = "Clipboard history",
        help = "Keep what you copy so the launcher can find it again. Kept in memory only and forgotten when sill stops; copies a password manager marks are never kept.",
        section = "Launcher",
        advanced
    )]
    pub clipboard_history: ClipboardHistory,
    /// `launcher.clipboard_cap` (M9).
    #[settings(
        label = "Clipboard history length",
        help = "How many copied items the history keeps; the oldest goes first.",
        section = "Launcher",
        range = "1..=500",
        advanced
    )]
    pub clipboard_cap: Count,
    /// `launcher.clipboard_image_max_mb` (M9).
    #[settings(
        label = "Largest copied image kept",
        help = "Copied images larger than this are not kept; 0 keeps no images.",
        section = "Launcher",
        range = "0..=64",
        unit = "MB",
        advanced
    )]
    pub clipboard_image_max_mb: Count,
    /// `launcher.emoji_skin_tone` (M9).
    #[settings(
        label = "Emoji skin tone",
        help = "The skin tone of emoji that have one.",
        section = "Launcher",
        advanced
    )]
    pub emoji_skin_tone: EmojiSkinTone,
    /// `launcher.web_search` (M9).
    #[settings(
        label = "Search the web",
        help = "Offer a web search for what you type, as the launcher's last result.",
        section = "Launcher",
        advanced
    )]
    pub web_search: WebSearch,
    /// `launcher.web_engine_url` (M9).
    #[settings(
        label = "Web search address",
        help = "The search engine's address, with {query} where the words go. Only https and http addresses are opened.",
        section = "Launcher",
        advanced
    )]
    pub web_engine_url: String,
    /// `launcher.currency` (M9).
    #[settings(
        label = "Currency conversion",
        help = "Convert currencies in the launcher's calculator with the European Central Bank's daily rates, fetched once a day; `off` fetches nothing.",
        section = "Launcher",
        advanced
    )]
    pub currency: CurrencyRates,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        LauncherSettings {
            top_bar_gap_px: Px(24),
            top_centre_bias_px: Px(230),
            top_vertical_fraction: Fraction(400),
            input_row_height_px: Px(56),
            result_row_height_px: Px(44),
            open_latency_budget_ms: Ms(100),
            material: PanelMaterial::Sheet,
            field_font_px: Px(22),
            field_font_weight: Count(500),
            field_glyph_px: Px(20),
            row_title_px: Px(14),
            row_detail_px: Px(12),
            files_backend: FilesBackend::Auto,
            files_budget_ms: Ms(150),
            clipboard_history: ClipboardHistory::Memory,
            clipboard_cap: Count(50),
            clipboard_image_max_mb: Count(8),
            emoji_skin_tone: EmojiSkinTone::Default,
            web_search: WebSearch::On,
            web_engine_url: "https://duckduckgo.com/?q={query}".to_owned(),
            currency: CurrencyRates::Off,
            extra: toml::Table::new(),
        }
    }
}
