//! `launcher.*` (design/22 §3.6; design/13 §13.3.9). All Advanced (§5).

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

use sill_launcher::SkinTone;
use sill_model::clipboard::HistoryMode;
use sill_model::on_off::OnOff;
use sill_model::results::CurrencyPolicy;

/// The launcher panel's placement and rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "launcher", page = Page::Appearance)]
pub struct LauncherSettings {
    /// `launcher.clipboard_history`.
    #[settings(
        label = "Clipboard history",
        help = "Keeps copies in memory only; password copies are never kept.",
        section = "Clipboard",
        advanced
    )]
    pub clipboard_history: HistoryMode,
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
    pub web_search: OnOff,
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
            clipboard_history: HistoryMode::Memory,
            emoji_skin_tone: SkinTone::Default,
            web_search: OnOff::On,
            currency: CurrencyPolicy::Off,
        }
    }
}
