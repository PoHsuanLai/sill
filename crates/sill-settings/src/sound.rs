//! `sound.*` (design/22 §3.11; design/13 §13.3.10). The two switches are on the Notifications
//! page (§5); the theme and event names are Advanced.
//!
//! `sound.theme` and the event names are plain `String`s: the schema derive recognises text by
//! the type name `String` only, so the doc's `SoundTheme(String)`/`SoundName(String)` newtypes
//! would be read as enums (FINDINGS.md, quire gap).

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `sound.ui_sounds`.
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
pub enum UiSounds {
    #[default]
    On,
    Off,
}

/// `sound.volume_feedback`.
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
pub enum VolumeFeedback {
    #[default]
    On,
    Off,
}

/// UI sounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "sound", page = Page::Notifications)]
pub struct SoundSettings {
    #[settings(
        label = "Sound theme",
        help = "The freedesktop sound theme UI sounds come from.",
        section = "Sounds",
        advanced
    )]
    pub theme: String,
    #[settings(
        label = "Play user interface sounds",
        help = "Sounds for screenshots, emptying the Trash and removing dock items.",
        section = "Sounds"
    )]
    pub ui_sounds: UiSounds,
    #[settings(
        label = "Play feedback when volume is changed",
        help = "A click on each volume key press.",
        section = "Sounds"
    )]
    pub volume_feedback: VolumeFeedback,
    #[settings(
        label = "Screenshot sound",
        help = "The sound name played for a screenshot.",
        section = "Sound names",
        advanced
    )]
    pub event_screenshot: String,
    #[settings(
        label = "Empty Trash sound",
        help = "The sound name played when the Trash is emptied.",
        section = "Sound names",
        advanced
    )]
    pub event_trash_empty: String,
    #[settings(
        label = "Volume step sound",
        help = "The sound name played on a volume key press.",
        section = "Sound names",
        advanced
    )]
    pub event_volume_step: String,
    #[settings(
        label = "Dock removal sound",
        help = "The sound name played when an icon is dragged out of the dock.",
        section = "Sound names",
        advanced
    )]
    pub event_dock_remove: String,
    #[settings(
        label = "Notification sound",
        help = "The sound name for a notification that names none.",
        section = "Sound names",
        advanced
    )]
    pub event_notification: String,
    #[settings(
        label = "Critical alert sound",
        help = "The sound name for a critical alert.",
        section = "Sound names",
        advanced
    )]
    pub event_critical_alert: String,
    #[settings(
        label = "Invalid key sound",
        help = "The sound name for a key an app cannot use.",
        section = "Sound names",
        advanced
    )]
    pub event_invalid_key: String,
    /// Keys this build does not know, kept for the next write.
    #[serde(flatten)]
    #[settings(skip)]
    pub extra: toml::Table,
}

impl Default for SoundSettings {
    fn default() -> Self {
        SoundSettings {
            theme: "freedesktop".to_owned(),
            ui_sounds: UiSounds::On,
            volume_feedback: VolumeFeedback::On,
            event_screenshot: "screen-capture".to_owned(),
            event_trash_empty: "trash-empty".to_owned(),
            event_volume_step: "audio-volume-change".to_owned(),
            event_dock_remove: "item-deleted".to_owned(),
            event_notification: "message-new-instant".to_owned(),
            event_critical_alert: "dialog-warning".to_owned(),
            event_invalid_key: "bell".to_owned(),
            extra: toml::Table::new(),
        }
    }
}
