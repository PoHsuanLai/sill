//! `osd.*`: the volume and brightness indicator (design/20 §1.7). design/22 has no rows for it
//! yet; `tests/keys.rs` lists them in `AWAITING_ROWS` and sill FINDINGS "OSD" has the row text.
//! All Advanced until design/22 §5 places them on a page.

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

/// `osd.enabled`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum OsdEnabled {
    #[default]
    On,
    Off,
}

/// `osd.position`: the output edge the indicator sits on, centred along it.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum OsdPosition {
    #[default]
    Bottom,
    Top,
}

/// The on-screen indicator for volume and brightness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "osd", page = Page::Appearance)]
pub struct OsdSettings {
    #[settings(
        label = "Show volume and brightness changes",
        help = "An indicator with the new level when the volume or brightness keys are pressed.",
        section = "Indicator",
        advanced
    )]
    pub enabled: OsdEnabled,
    #[settings(
        label = "Indicator stays for",
        help = "How long the indicator stays after the last change before it fades out.",
        section = "Indicator",
        range = "300..=10000",
        unit = "ms",
        advanced
    )]
    pub hold_ms: Ms,
    #[settings(
        label = "Indicator position",
        help = "The edge of the display the indicator sits on, centred along it.",
        section = "Indicator",
        advanced
    )]
    pub position: OsdPosition,
    #[settings(
        label = "Indicator distance from the edge",
        help = "The gap between the indicator and the dock (at the bottom) or the bar (at the top). The compositor already keeps it clear of the space they reserve.",
        section = "Indicator",
        range = "0..=400",
        unit = "px",
        advanced
    )]
    pub bottom_margin_px: Px,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for OsdSettings {
    fn default() -> Self {
        OsdSettings {
            enabled: OsdEnabled::On,
            hold_ms: Ms(1500),
            position: OsdPosition::Bottom,
            bottom_margin_px: Px(24),
            extra: toml::Table::new(),
        }
    }
}
