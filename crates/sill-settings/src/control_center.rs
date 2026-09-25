//! `control_center.*` (design/22 §3.13 and `control_center.material` from §3.1; design/13
//! §13.3.7). All Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Count, Px};
use serde::{Deserialize, Serialize};

/// Which quire material a panel uses. design/22 fixes Sheet for v1, but a one-variant enum
/// cannot be a settings key (the schema needs at least two words to pick a widget), and
/// design/20 §1.3 and §1.5 name Popover for the same panels, so both are offered (FINDINGS.md).
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
pub enum PanelMaterial {
    /// quire `Material::Sheet` (design/03 §17.3).
    #[default]
    Sheet,
    /// quire `Material::Popover` (design/20 §1.3, §1.5).
    Popover,
}

/// One control center module, in `control_center.modules` order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ControlCenterModule {
    Wifi,
    Bluetooth,
    Focus,
    Display,
    Sound,
    NowPlaying,
    Appearance,
    Battery,
}

/// The control center popover.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "control_center", page = Page::Appearance)]
pub struct ControlCenterSettings {
    #[settings(
        label = "Width",
        help = "The control center's width.",
        section = "Control center",
        range = "240..=480",
        unit = "px",
        advanced
    )]
    pub width_px: Px,
    #[settings(
        label = "Columns",
        help = "Module tiles per row.",
        section = "Control center",
        range = "1..=4",
        advanced
    )]
    pub grid_columns: Count,
    #[settings(
        label = "Tile gap",
        help = "Space between module tiles.",
        section = "Control center",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub grid_gap_px: Px,
    #[settings(
        label = "Padding",
        help = "Space around the module grid.",
        section = "Control center",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub grid_padding_px: Px,
    /// design/13 §13.3.7: the panel is at most `output_h - bar_h - bottom_margin_px` tall, then
    /// scrolls. No design/22 row yet (`AWAITING_ROWS`).
    #[settings(
        label = "Bottom margin",
        help = "The least space kept between the control center and the screen's bottom edge.",
        section = "Control center",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub bottom_margin_px: Px,
    #[settings(
        label = "Modules",
        help = "Which modules show, in order.",
        section = "Control center",
        advanced
    )]
    pub modules: Vec<ControlCenterModule>,
    /// `control_center.material` (§3.1 row, owned by sill: quire FINDINGS F15).
    #[settings(
        label = "Material",
        help = "The control center's material.",
        section = "Control center",
        advanced
    )]
    pub material: PanelMaterial,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for ControlCenterSettings {
    fn default() -> Self {
        use ControlCenterModule::{
            Appearance, Battery, Bluetooth, Display, Focus, NowPlaying, Sound, Wifi,
        };
        ControlCenterSettings {
            width_px: Px(320),
            grid_columns: Count(2),
            grid_gap_px: Px(8),
            grid_padding_px: Px(12),
            bottom_margin_px: Px(16),
            modules: vec![
                Wifi, Bluetooth, Focus, Display, Sound, NowPlaying, Appearance, Battery,
            ],
            material: PanelMaterial::Sheet,
            extra: toml::Table::new(),
        }
    }
}
