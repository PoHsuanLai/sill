//! `notifications.*` (design/22 §3.12 and `notifications.banner_material` from §3.1; design/13
//! §13.3.6). Do Not Disturb and the banner style are on the Notifications page (§5); the rest
//! are Advanced.

use crate::file::ShellFile;
use crate::section::Section;
use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Count, Fraction, Ms, Px, Scalar};
use serde::{Deserialize, Serialize};

/// `notifications.dnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Dnd {
    On,
    #[default]
    Off,
}

/// `notifications.banner_style`: the default for apps without their own choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum BannerStyle {
    /// Shows, then leaves by itself.
    #[default]
    Banner,
    /// Stays until acted on.
    Alert,
    /// History only.
    None,
}

/// `notifications.banner_entry_direction` (design/05 §12 open decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum BannerEntry {
    #[default]
    FromRight,
    FromBelow,
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

/// `notifications.swipe`: what swiping a banner away does. macOS keeps it in Notification
/// Center; only its close button, `CloseNotification` or Clear removes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NotificationSwipe {
    /// The banner leaves; the notification stays in the notification center.
    #[default]
    KeepInCenter,
    /// The notification is dismissed (NotificationClosed reason 2) and leaves the history.
    Dismiss,
}

/// `notifications.server`: whether sill serves `org.freedesktop.Notifications`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NotificationServer {
    /// Ask for the name, never replacing another daemon that holds it.
    #[default]
    Own,
    /// Leave the name to another daemon (cosmic-notifications, mako); sill shows nothing a
    /// sender posts.
    Passive,
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
        help = "No banners and no sounds, except critical ones.",
        section = "Notifications"
    )]
    pub dnd: Dnd,
    #[settings(
        label = "Notification style",
        help = "Banners leave by themselves; alerts stay until you act.",
        section = "Notifications"
    )]
    pub banner_style: BannerStyle,
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
        help = "The width of a banner.",
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
        help = "The size of an app icon in a banner.",
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
        label = "Banner entrance",
        help = "Which way banners slide in.",
        section = "Banners",
        advanced
    )]
    pub banner_entry_direction: BannerEntry,
    #[settings(
        label = "Time after hover",
        help = "The least time a banner stays after the pointer leaves it.",
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
        help = "How far a banner must be swiped to dismiss it.",
        section = "Swiping",
        range = "16..=400",
        unit = "px",
        advanced
    )]
    pub swipe_dismiss_px: Px,
    #[settings(
        label = "Swipe speed",
        help = "A swipe faster than this dismisses at any distance.",
        section = "Swiping",
        range = "0..=5000",
        unit = "px/s",
        advanced
    )]
    pub swipe_dismiss_velocity_px_s: Scalar,
    #[settings(
        label = "Leftward damping",
        help = "How much a leftward drag moves the banner (stored per-mille: 250 is 0.25).",
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
        help = "The banner's toast material, or ink on paper.",
        section = "Banners",
        advanced
    )]
    pub banner_material: BannerMaterial,
    #[settings(
        label = "Serve notifications",
        help = "Show other apps' notifications. Off leaves them to another notification daemon.",
        section = "Notifications",
        advanced
    )]
    pub server: NotificationServer,
    #[settings(
        label = "Swiping a banner away",
        help = "Keep the notification in the notification center, as macOS does, or dismiss it entirely.",
        section = "Notifications",
        advanced
    )]
    pub swipe: NotificationSwipe,
    #[settings(
        label = "Banner position",
        help = "The corner of the screen banners appear in.",
        section = "Banners",
        advanced
    )]
    pub banner_position: BannerPosition,
    #[settings(
        label = "Notifications kept",
        help = "The most notifications the notification center keeps; the oldest go first.",
        section = "Notification center",
        range = "0..=1000",
        advanced
    )]
    pub history_cap: Count,
    #[settings(
        label = "Notification center width",
        help = "The width of the notification center's column.",
        section = "Notification center",
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
            banner_style: BannerStyle::Banner,
            position_right_px: Px(8),
            position_top_gap_px: Px(8),
            banner_width_px: Px(360),
            banner_min_height_px: Px(64),
            banner_padding_px: Px(12),
            icon_px: Px(32),
            banner_hold_ms: Ms(5200),
            banner_entry_direction: BannerEntry::FromRight,
            hover_min_remaining_ms: Ms(1500),
            close_button_px: Px(18),
            swipe_dismiss_px: Px(80),
            swipe_dismiss_velocity_px_s: Scalar(600.0),
            swipe_damping: Fraction(250),
            stack_max: Count(3),
            stack_gap_px: Px(8),
            group_offset_px: Px(4),
            banner_material: BannerMaterial::Toast,
            server: NotificationServer::Own,
            swipe: NotificationSwipe::KeepInCenter,
            banner_position: BannerPosition::TopRight,
            history_cap: Count(100),
            center_width_px: Px(384),
        }
    }
}

impl Section for NotificationsSettings {
    const KEY: &'static str = "notifications";

    fn of(file: &ShellFile) -> &Self {
        &file.notifications
    }
}
