//! `menus.*` (design/22 §3.11; design/13 §13.3.2-13.3.4, §13.6). All Advanced (§5). The
//! switcher keys §3.11 groups with these are their own domain (`switcher.rs`): their dotted
//! paths are `switcher.*`, and the schema derive's path is always `<domain>.<field>`.

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
        help = "How long the pointer rests before a submenu opens.",
        section = "Menus",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub submenu_delay_ms: Ms,
    #[settings(
        label = "Item height",
        help = "How tall a menu item is.",
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
        label = "Submenu aim time",
        help = "Resting on the way to a submenu this long selects the item under it.",
        section = "Menus",
        range = "0..=2000",
        unit = "ms",
        advanced
    )]
    pub submenu_triangle_timeout_ms: Ms,
    #[settings(
        label = "First click",
        help = "A click right after a window activates only activates it.",
        section = "Windows",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub first_mouse_window_ms: Ms,
    /// `menus.font_px` (the macOS polish pass: 13 pt menu text).
    #[settings(
        label = "Text size",
        help = "The size of menu item text.",
        section = "Menus",
        range = "9..=24",
        unit = "px",
        advanced
    )]
    pub font_px: Px,
    /// `menus.highlight_radius_px`.
    #[settings(
        label = "Highlight radius",
        help = "How round the selected item's highlight is.",
        section = "Menus",
        range = "0..=12",
        unit = "px",
        advanced
    )]
    pub highlight_radius_px: Px,
    /// `menus.tooltip_font_px`.
    #[settings(
        label = "Tooltip text size",
        help = "The size of tooltip text.",
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
            item_height_px: Px(28),
            separator_margin_px: Px(5),
            submenu_triangle_timeout_ms: Ms(300),
            first_mouse_window_ms: Ms(100),
            font_px: Px(13),
            highlight_radius_px: Px(8),
            tooltip_font_px: Px(12),
        }
    }
}
