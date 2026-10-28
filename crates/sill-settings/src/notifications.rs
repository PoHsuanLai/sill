//! `notifications.*` (design/22 §3.12; design/13 §13.3.6). Do Not Disturb and the banner style are on the Notifications page (§5); the rest
//! are Advanced.

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::notifications::{Presentation, Swipe};
use sill_model::on_off::OnOff;

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
    pub dnd: OnOff,
    #[settings(
        label = "Notification style",
        help = "Banners leave on their own; alerts stay until you act.",
        section = "Alerts"
    )]
    pub banner_style: Presentation,
    #[settings(
        label = "Swiping a banner away",
        help = "Keeps it in the notification center, or dismisses it.",
        section = "Swiping",
        advanced
    )]
    pub swipe: Swipe,
}

impl Default for NotificationsSettings {
    fn default() -> Self {
        NotificationsSettings {
            dnd: OnOff::Off,
            banner_style: Presentation::Banner,
            swipe: Swipe::KeepInCenter,
        }
    }
}
