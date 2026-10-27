//! `notifications.*` (design/22 §3.12; design/13 §13.3.6). Do Not Disturb and the banner style are on the Notifications page (§5); the rest
//! are Advanced.

use ds_core::word::Word;
use ds_settings::Px;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::notifications::{Presentation, Swipe};

/// `notifications.dnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Dnd {
    On,
    #[default]
    Off,
}

/// Notification banners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "notifications", page = Page::Notifications)]
pub struct NotificationsSettings {
    #[settings(
        label = "Do Not Disturb",
        help = "No banners or sounds, except critical ones.",
        section = "Alerts"
    )]
    pub dnd: Dnd,
    #[settings(
        label = "Notification style",
        help = "Banners leave on their own; alerts stay until you act.",
        section = "Alerts"
    )]
    pub banner_style: Presentation,
    #[settings(
        label = "Right margin",
        help = "Space between banners and the screen's right edge.",
        section = "Banners",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub position_right_px: Px,
    #[settings(
        label = "Top margin",
        help = "Space between the bar and the first banner.",
        section = "Banners",
        range = "0..=64",
        unit = "px",
        advanced
    )]
    pub position_top_gap_px: Px,
    #[settings(
        label = "Banner width",
        help = "How wide a banner is.",
        section = "Banners",
        range = "200..=600",
        unit = "px",
        advanced
    )]
    pub banner_width_px: Px,
    #[settings(
        label = "Icon size",
        help = "The size of the app icon in a banner.",
        section = "Banners",
        range = "16..=64",
        unit = "px",
        advanced
    )]
    pub icon_px: Px,
    #[settings(
        label = "Swiping a banner away",
        help = "Keeps it in the notification center, or dismisses it.",
        section = "Swiping",
        advanced
    )]
    pub swipe: Swipe,
    #[settings(
        label = "Center width",
        help = "How wide the notification center is.",
        section = "History",
        range = "280..=600",
        unit = "px",
        advanced
    )]
    pub center_width_px: Px,
}

impl Default for NotificationsSettings {
    fn default() -> Self {
        NotificationsSettings {
            dnd: Dnd::Off,
            banner_style: Presentation::Banner,
            position_right_px: Px(8),
            position_top_gap_px: Px(8),
            banner_width_px: Px(320),
            icon_px: Px(28),
            swipe: Swipe::KeepInCenter,
            center_width_px: Px(320),
        }
    }
}
