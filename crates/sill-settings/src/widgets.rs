//! `widgets.*`: which widgets the notification center shows and in what order, and the desktop
//! widget layer (design/20 §1.14). macOS keeps both on its "Desktop & Dock" page, hence
//! `Page::Dock`. All Advanced.
//!
//! Each widget has one size: nothing here picks one.
//!
//! Where a widget sits on the desktop is saved state (like the dock's pins), not a setting:
//! `$XDG_STATE_HOME/sill/desktop-widgets.json`, owned by the desktop layer.

use ds_core::word::Word;
use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// One widget. The same component draws it in the notification center and on the desktop. Weather
/// is not a variant yet: it needs a weather source nothing provides; adding it later is additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    /// The month, today marked (macOS's Calendar widget, month size).
    Calendar,
    /// The next events from the calendar sources (macOS's Calendar "Up Next").
    UpNext,
    /// The battery of this machine and of every Bluetooth device that reports one.
    Battery,
    /// The current MPRIS player (the control center's Now Playing module).
    NowPlaying,
    /// The time in the zones `widgets.world_clocks` names.
    WorldClock,
}

/// `widgets.desktop`: whether the desktop widget layer is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum DesktopWidgets {
    #[default]
    Off,
    On,
}

/// The widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "widgets", page = Page::Dock)]
pub struct WidgetsSettings {
    #[settings(
        label = "Notification Center widgets",
        help = "Which widgets show under the notifications, in order.",
        section = "Widgets",
        advanced
    )]
    pub center: Vec<WidgetKind>,
    #[settings(
        label = "Show widgets on the desktop",
        help = "Widgets on the desktop, under the windows (`sill widgets toggle-desktop`).",
        section = "Widgets",
        advanced
    )]
    pub desktop: DesktopWidgets,
    #[settings(
        label = "Desktop widgets",
        help = "Which widgets the desktop shows, placed on a grid and movable; an output that has been through Edit Widgets keeps a list of its own.",
        section = "Widgets",
        advanced
    )]
    pub desktop_widgets: Vec<WidgetKind>,
    #[settings(
        label = "Desktop grid cell",
        help = "The side of one cell of the desktop's widget grid; a small widget is one cell.",
        section = "Widgets",
        range = "120..=240",
        unit = "px",
        advanced
    )]
    pub desktop_cell_px: Px,
    #[settings(
        label = "Desktop grid gap",
        help = "The space between cells of the desktop's widget grid and around it.",
        section = "Widgets",
        range = "0..=48",
        unit = "px",
        advanced
    )]
    pub desktop_gap_px: Px,
    /// `;`-separated IANA zone names; a `Vec` of text is not derivable in the settings macro
    /// yet (`display.scale_overrides` does the same).
    #[settings(
        label = "World clock cities",
        help = "Time zones for the World Clock widget, separated by `;`: `Europe/London; America/New_York`.",
        section = "Widgets",
        advanced
    )]
    pub world_clocks: String,
}

impl Default for WidgetsSettings {
    fn default() -> Self {
        use WidgetKind::{Battery, Calendar, UpNext};
        WidgetsSettings {
            center: vec![Calendar, UpNext, Battery],
            desktop: DesktopWidgets::Off,
            desktop_widgets: vec![Calendar, Battery],
            desktop_cell_px: Px(164),
            desktop_gap_px: Px(16),
            world_clocks: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetKind;
    use ds_core::word::Word;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Keys {
        kinds: Vec<WidgetKind>,
    }

    #[test]
    fn kinds_are_stored_by_their_snake_case_keys() {
        let keys = Keys {
            kinds: WidgetKind::ALL.to_vec(),
        };
        let text = toml::to_string(&keys).unwrap();
        assert_eq!(
            text.trim(),
            r#"kinds = ["calendar", "up_next", "battery", "now_playing", "world_clock"]"#
        );
        assert_eq!(toml::from_str::<Keys>(&text).unwrap(), keys);
    }

    #[test]
    fn slugs_are_the_kebab_case_element_id_suffixes() {
        let slugs: Vec<&str> = WidgetKind::ALL.iter().map(|kind| kind.slug()).collect();
        assert_eq!(
            slugs,
            [
                "calendar",
                "up-next",
                "battery",
                "now-playing",
                "world-clock"
            ]
        );
    }
}
