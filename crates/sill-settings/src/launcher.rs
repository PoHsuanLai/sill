//! `launcher.*` (design/22 §3.6 and `launcher.material` from §3.1; design/13 §13.3.9). All
//! Advanced (§5).

use ds_settings::schema::Page;
use ds_settings::{Fraction, Ms, Px};
use serde::{Deserialize, Serialize};

use crate::control_center::PanelMaterial;

/// The launcher panel's placement and rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "launcher", page = Page::Appearance)]
pub struct LauncherSettings {
    /// Panel top = `max(bar_h + top_bar_gap, round(top_vertical_fraction x H - top_centre_bias))`.
    #[settings(
        label = "Gap below the bar",
        help = "The least space between the bar and the launcher.",
        section = "Launcher",
        range = "0..=200",
        unit = "px",
        advanced
    )]
    pub top_bar_gap_px: Px,
    #[settings(
        label = "Height bias",
        help = "Subtracted from the launcher's vertical position.",
        section = "Launcher",
        range = "0..=1000",
        unit = "px",
        advanced
    )]
    pub top_centre_bias_px: Px,
    #[settings(
        label = "Vertical position",
        help = "Where the launcher sits, as a fraction of the screen's height (stored per-mille).",
        section = "Launcher",
        range = "0..=1000",
        advanced
    )]
    pub top_vertical_fraction: Fraction,
    #[settings(
        label = "Search row height",
        help = "The height of the search field row.",
        section = "Launcher",
        range = "32..=96",
        unit = "px",
        advanced
    )]
    pub input_row_height_px: Px,
    #[settings(
        label = "Result row height",
        help = "The height of one result row.",
        section = "Launcher",
        range = "24..=96",
        unit = "px",
        advanced
    )]
    pub result_row_height_px: Px,
    #[settings(
        label = "Open latency budget",
        help = "The p95 open time the acceptance script holds the launcher to.",
        section = "Launcher",
        range = "10..=1000",
        unit = "ms",
        advanced
    )]
    pub open_latency_budget_ms: Ms,
    /// `launcher.material` (§3.1 row, owned by sill: quire FINDINGS F15).
    #[settings(
        label = "Panel material",
        help = "The launcher panel's material.",
        section = "Launcher",
        advanced
    )]
    pub material: PanelMaterial,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        LauncherSettings {
            top_bar_gap_px: Px(24),
            top_centre_bias_px: Px(230),
            top_vertical_fraction: Fraction(400),
            input_row_height_px: Px(56),
            result_row_height_px: Px(44),
            open_latency_budget_ms: Ms(100),
            material: PanelMaterial::Sheet,
            extra: toml::Table::new(),
        }
    }
}
