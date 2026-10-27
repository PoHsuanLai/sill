//! `scrollbar.*` (design/22 §3.8; design/11 §11.3.12): the overlay thumb shell-host paints.
//! All Advanced (§5).

use ds_core::word::Word;
use ds_settings::Px;
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
        label = "Track width",
        help = "The track's width at rest.",
        section = "Scroll bars",
        range = "4..=32",
        unit = "px",
        advanced
    )]
    pub track_thickness_px: Px,
    #[settings(
        label = "Track width on hover",
        help = "The track's width while the pointer is near.",
        section = "Scroll bars",
        range = "4..=32",
        unit = "px",
        advanced
    )]
    pub track_thickness_hover_px: Px,
    #[settings(
        label = "Thumb width",
        help = "The thumb's width at rest.",
        section = "Scroll bars",
        range = "2..=32",
        unit = "px",
        advanced
    )]
    pub thumb_thickness_px: Px,
    #[settings(
        label = "Thumb width on hover",
        help = "The thumb's width while the pointer is near.",
        section = "Scroll bars",
        range = "2..=32",
        unit = "px",
        advanced
    )]
    pub thumb_thickness_hover_px: Px,
    #[settings(
        label = "Shortest thumb",
        help = "The thumb never gets shorter than this.",
        section = "Scroll bars",
        range = "8..=64",
        unit = "px",
        advanced
    )]
    pub thumb_min_length_px: Px,
    #[settings(
        label = "Always-on track width",
        help = "The track's width when scroll bars always show.",
        section = "Scroll bars",
        range = "8..=32",
        unit = "px",
        advanced
    )]
    pub legacy_track_width_px: Px,
    #[settings(
        label = "Corner clearance",
        help = "How far the vertical track stops short of a horizontal one.",
        section = "Scroll bars",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub other_axis_offset_px: Px,
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
            track_thickness_px: Px(12),
            track_thickness_hover_px: Px(16),
            thumb_thickness_px: Px(8),
            thumb_thickness_hover_px: Px(12),
            thumb_min_length_px: Px(18),
            legacy_track_width_px: Px(15),
            other_axis_offset_px: Px(12),
            track_click_mode: TrackClick::PageTowardClick,
        }
    }
}
