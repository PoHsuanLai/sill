//! `menus.*` (design/22 §3.11; design/13 §13.3.2-13.3.4, §13.6). All Advanced (§5). The
//! switcher keys §3.11 groups with these are their own domain (`switcher.rs`): their dotted
//! paths are `switcher.*`, and the schema derive's path is always `<domain>.<field>`.

use crate::file::ShellFile;
use crate::section::Section;
use ds_settings::schema::Page;
use ds_settings::{Ms, Px};
use serde::{Deserialize, Serialize};

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
        label = "Separator margin",
        help = "Space above and below a separator.",
        section = "Menus",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub separator_margin_px: Px,
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
        label = "First click window",
        help = "A click this soon after a window activates only activates it.",
        section = "Windows",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub first_mouse_window_ms: Ms,
    /// `menus.font_px` (the macOS polish pass: 13 pt menu text).
    #[settings(
        label = "Text size",
        help = "The size of a text menu's items.",
        section = "Menus",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub font_px: Px,
    /// `menus.highlight_radius_px`.
    #[settings(
        label = "Highlight radius",
        help = "The corner radius of the selected item's highlight.",
        section = "Menus",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub highlight_radius_px: Px,
    /// `menus.tooltip_font_px`.
    #[settings(
        label = "Tooltip text size",
        help = "The size of a tooltip's label.",
        section = "Menus",
        range = "9..=20",
        unit = "px",
        advanced
    )]
    pub tooltip_font_px: Px,
}

impl Default for MenusSettings {
    fn default() -> Self {
        MenusSettings {
            submenu_delay_ms: Ms(200),
            item_height_px: Px(22),
            separator_margin_px: Px(5),
            submenu_triangle_timeout_ms: Ms(300),
            first_mouse_window_ms: Ms(100),
            font_px: Px(13),
            highlight_radius_px: Px(6),
            tooltip_font_px: Px(12),
        }
    }
}

impl Section for MenusSettings {
    const KEY: &'static str = "menus";

    fn of(file: &ShellFile) -> &Self {
        &file.menus
    }
}
