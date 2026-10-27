//! `screenshot.*`: the floating thumbnail after a screenshot (design/20 §1.13), which folders are
//! watched for new screenshots, and what `sill screenshot` runs. macOS keeps its screenshot
//! shortcuts under Keyboard, hence `Page::KeyboardAndShortcuts`. All Advanced.

use ds_core::word::Word;
use ds_settings::Ms;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::screenshots::ShotTool;

/// Whether a new screenshot shows its floating thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum ShotThumbnail {
    #[default]
    Show,
    Hide,
}

/// Screenshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "screenshot", page = Page::KeyboardAndShortcuts)]
pub struct ScreenshotSettings {
    #[settings(
        label = "Show floating thumbnail",
        help = "Click it to open, drag it into a document, or swipe it away.",
        section = "Screenshots",
        advanced
    )]
    pub thumbnail: ShotThumbnail,
    #[settings(
        label = "Thumbnail stays for",
        help = "How long it stays unless the pointer is on it.",
        section = "Screenshots",
        range = "1000..=30000",
        unit = "ms",
        advanced
    )]
    pub thumbnail_hold_ms: Ms,
    /// `auto`, `off`, or `;`-separated folders (`sill_model::screenshots::parse_folders`).
    #[settings(
        label = "Screenshot folders",
        help = "Auto uses your Pictures folders; off watches none; or list folders separated by ;.",
        section = "Screenshots",
        advanced
    )]
    pub folders: String,
    /// `auto`, `off`, or a command line, parsed at load (`sill_model::screenshots::parse_tool`).
    #[settings(
        label = "Screenshot tool",
        help = "Auto uses the system's; off runs nothing; or enter your own command.",
        section = "Screenshots",
        text,
        advanced
    )]
    pub tool: ShotTool,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        ScreenshotSettings {
            thumbnail: ShotThumbnail::Show,
            thumbnail_hold_ms: Ms(5200),
            folders: "auto".to_owned(),
            tool: ShotTool::Portal,
        }
    }
}
