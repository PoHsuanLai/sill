//! `dock.*` (design/22 §3.5, §4.1; design/10 §10.6). The Dock page shows the keys §5
//! lists; the geometry keys are Advanced. Timing is constants in the dock's crates. Pinned apps are placements in `layout.toml`, not
//! here.

use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

pub use crate::dock_words::{AutoHide, DockFloor, Indicators, Magnification, TrashTile};

/// The dock's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "dock", page = Page::Dock)]
pub struct DockSettings {
    #[settings(
        label = "Size",
        help = "How big icons are at rest.",
        section = "Dock",
        range = "32..=80",
        unit = "px"
    )]
    pub tile_size_px: Px,
    #[settings(
        label = "Magnification",
        help = "Icons grow under the pointer.",
        section = "Dock"
    )]
    pub magnification: Magnification,
    #[settings(
        label = "Magnified size",
        help = "How big the icon under the pointer gets.",
        section = "Dock",
        range = "32..=72",
        unit = "px",
        advanced
    )]
    pub magnified_size_px: Px,
    #[settings(
        label = "Icon gap",
        help = "Space between icons.",
        section = "Dock",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub tile_gap_px: Px,
    #[settings(
        label = "Automatically hide the Dock",
        help = "Shows the Dock when the pointer reaches the bottom.",
        section = "Hiding"
    )]
    pub autohide: AutoHide,
    #[settings(
        label = "Show indicators for open apps",
        help = "A dot under each app with a window.",
        section = "Behaviour"
    )]
    pub indicators: Indicators,
    #[settings(
        label = "Show Trash",
        help = "Adds a Trash tile at the end.",
        section = "Behaviour"
    )]
    pub trash: TrashTile,
    #[settings(
        label = "Corner radius",
        help = "How round the Dock's background is.",
        section = "Dock",
        range = "0..=48",
        unit = "px",
        advanced
    )]
    pub pill_radius_px: Px,
    #[settings(
        label = "Edge margin",
        help = "How close the Dock gets to the screen's sides.",
        section = "Dock",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub edge_clamp_px: Px,
    #[settings(
        label = "Indicator dot",
        help = "How big the open-app dot is.",
        section = "Behaviour",
        range = "2..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_diameter_px: Px,
    #[settings(
        label = "Separator height",
        help = "The height of the line before Trash.",
        section = "Behaviour",
        range = "8..=80",
        unit = "px",
        advanced
    )]
    pub separator_height_px: Px,
    /// `dock.pill_padding_px` (the macOS polish pass: 6 on every side of the tiles).
    #[settings(
        label = "Dock padding",
        help = "Space between the icons and the Dock's edge.",
        section = "Dock",
        range = "0..=24",
        unit = "px",
        advanced
    )]
    pub pill_padding_px: Px,
    /// `dock.running_dot_gap_px`.
    #[settings(
        label = "Indicator dot gap",
        help = "Space from an icon's bottom to its dot.",
        section = "Behaviour",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub running_dot_gap_px: Px,
    /// `dock.floor`.
    #[settings(
        label = "Floor",
        help = "A soft light band under the icons.",
        section = "Dock",
        advanced
    )]
    pub floor: DockFloor,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            magnification: Magnification::On,
            tile_size_px: Px(48),
            magnified_size_px: Px(72),
            tile_gap_px: Px(8),
            autohide: AutoHide::Off,
            indicators: Indicators::On,
            trash: TrashTile::On,
            pill_radius_px: Px(22),
            edge_clamp_px: Px(8),
            running_dot_diameter_px: Px(4),
            separator_height_px: Px(36),
            pill_padding_px: Px(6),
            running_dot_gap_px: Px(3),
            floor: DockFloor::Off,
        }
    }
}
