//! `osd.*`: the volume and brightness indicator (design/20 §1.7). All Advanced until design/22 §5
//! places them on a page.

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

/// `osd.enabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum OsdEnabled {
    #[default]
    On,
    Off,
}

/// `osd.position`: the output edge the indicator sits on, centred along it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum OsdPosition {
    /// Top right under the bar, as current macOS shows its volume and brightness panel.
    #[default]
    TopRight,
    /// Centred above the dock.
    BottomCentre,
}

/// The on-screen indicator for volume and brightness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "osd", page = Page::Appearance)]
pub struct OsdSettings {
    #[settings(
        label = "Show volume and brightness changes",
        help = "Shows the new level when you press the keys.",
        section = "Indicator",
        advanced
    )]
    pub enabled: OsdEnabled,
    #[settings(
        label = "Indicator stays for",
        help = "How long it stays after the last change.",
        section = "Indicator",
        range = "300..=10000",
        unit = "ms",
        advanced
    )]
    pub hold_ms: Ms,
    #[settings(
        label = "Indicator position",
        help = "Which screen edge it sits on.",
        section = "Indicator",
        advanced
    )]
    pub position: OsdPosition,
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
        OsdSettings {
            enabled: OsdEnabled::On,
            hold_ms: Ms(1500),
            position: OsdPosition::TopRight,
            margin_px: Px(24),
        }
    }
}
