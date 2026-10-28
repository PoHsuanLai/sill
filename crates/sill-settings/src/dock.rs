//! `dock.*` (design/22 §3.5, §4.1; design/10 §10.6). The Dock page shows the keys §5
//! lists; the magnified size is Advanced. Timing is constants in the dock's crates, and the rest of
//! the geometry (gap, padding, corner, dot, separator) is stylesheet tokens (`DockToken`). Pinned
//! apps are placements in `layout.toml`, not here.

use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

use sill_model::on_off::OnOff;

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
    pub magnification: OnOff,
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
        label = "Automatically hide the Dock",
        help = "Shows the Dock when the pointer reaches the bottom.",
        section = "Hiding"
    )]
    pub autohide: OnOff,
    #[settings(
        label = "Show indicators for open apps",
        help = "A dot under each app with a window.",
        section = "Behaviour"
    )]
    pub indicators: OnOff,
    #[settings(
        label = "Show Trash",
        help = "Adds a Trash tile at the end.",
        section = "Behaviour"
    )]
    pub trash: OnOff,
    /// `dock.floor`.
    #[settings(
        label = "Floor",
        help = "A soft light band under the icons.",
        section = "Dock",
        advanced
    )]
    pub floor: OnOff,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            magnification: OnOff::On,
            tile_size_px: Px(48),
            magnified_size_px: Px(72),
            autohide: OnOff::Off,
            indicators: OnOff::On,
            trash: OnOff::On,
            floor: OnOff::Off,
        }
    }
}
