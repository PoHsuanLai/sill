//! `scrollbar.*` (design/22 §3.8; design/11 §11.3.12): the overlay thumb shell-host paints.
//! All Advanced (§5).

use ds_core::word::Word;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `scrollbar.track_click_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum TrackClick {
    /// A click on the track pages toward it (macOS default).
    #[default]
    PageTowardClick,
    /// A click on the track jumps there.
    JumpToClick,
}

/// The overlay scrollbar's geometry and timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "scrollbar", page = Page::MouseAndGestures)]
pub struct ScrollbarSettings {
    #[settings(
        label = "Clicking the track",
        help = "Pages toward the click, or jumps to it.",
        section = "Scroll bars",
        advanced
    )]
    pub track_click_mode: TrackClick,
}

impl Default for ScrollbarSettings {
    fn default() -> Self {
        ScrollbarSettings {
            track_click_mode: TrackClick::PageTowardClick,
        }
    }
}
