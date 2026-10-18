//! `control_center.*` (design/22 §3.13; design/13
//! §13.3.7). All Advanced (§5).

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Count, Px};
use serde::{Deserialize, Serialize};

/// Which quire material a panel uses. design/22 fixes Sheet for v1, but a one-variant enum
/// cannot be a settings key (the schema needs at least two words to pick a widget), and
/// design/20 §1.3 and §1.5 name Popover for the same panels, so both are offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum PanelMaterial {
    /// quire `Material::Sheet` (design/03 §17.3).
    #[default]
    Sheet,
    /// quire `Material::Popover` (design/20 §1.3, §1.5).
    Popover,
}

/// Whether a module also shows as its own bar item with its own dropdown (macOS's "Show in Menu
/// Bar"), beside the control center's own item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum InMenuBar {
    Show,
    #[default]
    Hide,
}

/// One control center module, in `control_center.modules` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum ControlCenterModule {
    Wifi,
    Bluetooth,
    Focus,
    Display,
    /// The keyboard's backlight (design/26 G25); drawn only where the machine has one.
    KeyboardBrightness,
    Sound,
    NowPlaying,
    Appearance,
    Battery,
    /// What the terminal may do without asking, and the way back from a pause (intentd's
    /// `Control`); drawn only while it has a grant or a pause to show.
    Terminal,
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
    /// scrolls.
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
    /// Whether Wi-Fi also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Wi-Fi in the menu bar",
        help = "Show Wi-Fi as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_wifi: InMenuBar,
    /// Whether Bluetooth also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Bluetooth in the menu bar",
        help = "Show Bluetooth as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_bluetooth: InMenuBar,
    /// Whether Sound also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Sound in the menu bar",
        help = "Show Sound as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_sound: InMenuBar,
    /// Whether Display also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Display in the menu bar",
        help = "Show Display as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_display: InMenuBar,
    /// Whether Battery also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Battery in the menu bar",
        help = "Show Battery as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_battery: InMenuBar,
    /// Whether Now Playing also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Now Playing in the menu bar",
        help = "Show Now Playing as its own item in the menu bar, beside the control center.",
        section = "Menu bar items",
        advanced
    )]
    pub menu_bar_now_playing: InMenuBar,
}

impl Default for ControlCenterSettings {
    fn default() -> Self {
        use ControlCenterModule::{
            Appearance, Battery, Bluetooth, Display, Focus, KeyboardBrightness, NowPlaying, Sound,
            Terminal, Wifi,
        };
        ControlCenterSettings {
            width_px: Px(320),
            grid_columns: Count(2),
            grid_gap_px: Px(10),
            grid_padding_px: Px(10),
            bottom_margin_px: Px(16),
            modules: vec![
                Wifi,
                Bluetooth,
                Focus,
                Display,
                KeyboardBrightness,
                Sound,
                NowPlaying,
                Appearance,
                Battery,
                Terminal,
            ],
            menu_bar_wifi: InMenuBar::Show,
            menu_bar_bluetooth: InMenuBar::Hide,
            menu_bar_sound: InMenuBar::Show,
            menu_bar_display: InMenuBar::Hide,
            menu_bar_battery: InMenuBar::Show,
            menu_bar_now_playing: InMenuBar::Hide,
        }
    }
}
