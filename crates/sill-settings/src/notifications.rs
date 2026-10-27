//! `notifications.*` (design/22 §3.12 and `notifications.banner_material` from §3.1; design/13
//! §13.3.6). Do Not Disturb and the banner style are on the Notifications page (§5); the rest
//! are Advanced.

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Count, Fraction, Ms, Px, Scalar};
use serde::{Deserialize, Serialize};
use sill_model::notifications::{Presentation, ServerMode, Swipe};

/// `notifications.dnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Dnd {
    On,
    #[default]
    Off,
}

/// `notifications.banner_material` (design/13 §13.9 open decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum BannerMaterial {
    /// quire `Material::Toast`.
    #[default]
    Toast,
    /// Ink on paper, like mail's toast.
    Inverse,
}

/// `notifications.banner_position`: the output corner banners stack in (design/20 §3 open
/// decision 2: top right as macOS, or bottom right nearer mail's toast).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum BannerPosition {
    /// Under the bar, newest on top (macOS).
    #[default]
    TopRight,
    /// Above the dock's reserve, newest at the bottom.
    BottomRight,
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
        label = "Banner height",
        help = "The least height of a banner.",
        section = "Banners",
        range = "32..=200",
        unit = "px",
        advanced
    )]
    pub banner_min_height_px: Px,
    #[settings(
        label = "Banner padding",
        help = "Space inside a banner.",
        section = "Banners",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub banner_padding_px: Px,
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
        label = "Banner time",
        help = "How long a banner stays.",
        section = "Banners",
        range = "1000..=30000",
        unit = "ms",
        advanced
    )]
    pub banner_hold_ms: Ms,
    #[settings(
        label = "Time after hover",
        help = "The least time a banner stays after the pointer leaves.",
        section = "Banners",
        range = "0..=10000",
        unit = "ms",
        advanced
    )]
    pub hover_min_remaining_ms: Ms,
    #[settings(
        label = "Close button",
        help = "The size of a banner's close button.",
        section = "Banners",
        range = "12..=32",
        unit = "px",
        advanced
    )]
    pub close_button_px: Px,
    #[settings(
        label = "Swipe distance",
        help = "How far to swipe to dismiss a banner.",
        section = "Swiping",
        range = "16..=400",
        unit = "px",
        advanced
    )]
    pub swipe_dismiss_px: Px,
    #[settings(
        label = "Swipe speed",
        help = "A faster swipe dismisses from any distance.",
        section = "Swiping",
        range = "0..=5000",
        unit = "px/s",
        advanced
    )]
    pub swipe_dismiss_velocity_px_s: Scalar,
    #[settings(
        label = "Leftward damping",
        help = "How far a leftward drag moves it (250 is 0.25).",
        section = "Swiping",
        range = "0..=1000",
        advanced
    )]
    pub swipe_damping: Fraction,
    #[settings(
        label = "Visible banners",
        help = "The most banners shown at once.",
        section = "Banners",
        range = "1..=10",
        advanced
    )]
    pub stack_max: Count,
    #[settings(
        label = "Banner gap",
        help = "Space between stacked banners.",
        section = "Banners",
        range = "0..=32",
        unit = "px",
        advanced
    )]
    pub stack_gap_px: Px,
    #[settings(
        label = "Group offset",
        help = "The offset of the layers under a grouped banner.",
        section = "Banners",
        range = "0..=16",
        unit = "px",
        advanced
    )]
    pub group_offset_px: Px,
    /// `notifications.banner_material` (§3.1 row, owned by sill).
    #[settings(
        label = "Banner material",
        help = "The banner's look: toast, or ink on paper.",
        section = "Banners",
        advanced
    )]
    pub banner_material: BannerMaterial,
    #[settings(
        label = "Show other apps' notifications",
        help = "Off leaves them to another app.",
        section = "Alerts",
        advanced
    )]
    pub server: ServerMode,
    #[settings(
        label = "Swiping a banner away",
        help = "Keeps it in the notification center, or dismisses it.",
        section = "Swiping",
        advanced
    )]
    pub swipe: Swipe,
    #[settings(
        label = "Banner position",
        help = "Which screen corner banners appear in.",
        section = "Banners",
        advanced
    )]
    pub banner_position: BannerPosition,
    #[settings(
        label = "Notifications kept",
        help = "The most the notification center keeps; the oldest go first.",
        section = "History",
        range = "0..=1000",
        advanced
    )]
    pub history_cap: Count,
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
            banner_min_height_px: Px(48),
            banner_padding_px: Px(10),
            icon_px: Px(28),
            banner_hold_ms: Ms(5200),
            hover_min_remaining_ms: Ms(1500),
            close_button_px: Px(18),
            swipe_dismiss_px: Px(80),
            swipe_dismiss_velocity_px_s: Scalar(600.0),
            swipe_damping: Fraction(250),
            stack_max: Count(3),
            stack_gap_px: Px(8),
            group_offset_px: Px(4),
            banner_material: BannerMaterial::Toast,
            server: ServerMode::Own,
            swipe: Swipe::KeepInCenter,
            banner_position: BannerPosition::TopRight,
            history_cap: Count(100),
            center_width_px: Px(320),
        }
    }
}
