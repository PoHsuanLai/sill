//! `menus.*` (design/22 §3.11; design/13 §13.3.2-13.3.4, §13.6). All Advanced (§5). The
//! switcher and sound keys §3.11 groups with these are their own domains (`switcher.rs`,
//! `sound.rs`): their dotted paths are `switcher.*` and `sound.*`, and the schema derive's path
//! is always `<domain>.<field>` (FINDINGS.md).

use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

/// `menus.pick_feedback` (design/13 §13.9 open decision 3).
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
pub enum PickFeedback {
    /// Close, then pick (the design).
    #[default]
    None,
    /// Blink the chosen item once, then close (macOS).
    BlinkOnce,
}

/// Every text menu's tracking and geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "menus", page = Page::Appearance)]
pub struct MenusSettings {
    #[settings(
        label = "Submenu delay",
        help = "How long the pointer rests on an item before its submenu opens.",
        section = "Menus",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub submenu_delay_ms: Ms,
    #[settings(
        label = "Item height",
        help = "The height of one menu item.",
        section = "Menus",
        range = "18..=40",
        unit = "px",
        advanced
    )]
    pub item_height_px: Px,
    #[settings(
        label = "Widest menu",
        help = "Menus never grow wider than this.",
        section = "Menus",
        range = "220..=800",
        unit = "px",
        advanced
    )]
    pub max_width_px: Px,
    #[settings(
        label = "Separator margin",
        help = "Space above and below a separator.",
        section = "Menus",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub separator_margin_px: Px,
    #[settings(
        label = "Header height",
        help = "The height of a section header.",
        section = "Menus",
        range = "12..=40",
        unit = "px",
        advanced
    )]
    pub section_header_height_px: Px,
    #[settings(
        label = "Safe triangle timeout",
        help = "Resting inside the path to a submenu this long selects what is under the pointer.",
        section = "Menus",
        range = "0..=2000",
        unit = "ms",
        advanced
    )]
    pub submenu_triangle_timeout_ms: Ms,
    #[settings(
        label = "Pick feedback",
        help = "Blink the chosen item before the menu closes.",
        section = "Menus",
        advanced
    )]
    pub pick_feedback: PickFeedback,
    #[settings(
        label = "First click window",
        help = "A click this soon after a window activates only activates it.",
        section = "Windows",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub first_mouse_window_ms: Ms,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for MenusSettings {
    fn default() -> Self {
        MenusSettings {
            submenu_delay_ms: Ms(200),
            item_height_px: Px(24),
            max_width_px: Px(420),
            separator_margin_px: Px(4),
            section_header_height_px: Px(22),
            submenu_triangle_timeout_ms: Ms(300),
            pick_feedback: PickFeedback::None,
            first_mouse_window_ms: Ms(100),
            extra: toml::Table::new(),
        }
    }
}
