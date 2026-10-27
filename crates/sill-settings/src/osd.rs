//! `osd.*`: the volume and brightness indicator (design/20 §1.7). Its hold and place are constants
//! in `sill-overlays`; the gap is Advanced until design/22 §5 places it on a page.

use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// The on-screen indicator for volume and brightness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "osd", page = Page::Appearance)]
pub struct OsdSettings {
    #[settings(
        label = "Distance from the edge",
        help = "Space from the bar, Dock or right edge.",
        section = "Indicator",
        range = "0..=400",
        unit = "px",
        advanced
    )]
    pub margin_px: Px,
}

impl Default for OsdSettings {
    fn default() -> Self {
        OsdSettings { margin_px: Px(24) }
    }
}
