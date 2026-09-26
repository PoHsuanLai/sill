//! The launcher v2 keys' value types (M9 freeze, sill FINDINGS "M9 freeze" F652).

use serde::{Deserialize, Serialize};

/// `launcher.files_backend`: which file index the launcher's Documents search asks.
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
pub enum FilesBackend {
    /// Tracker 3 (localsearch) when it runs, else Baloo, else `fd` or `plocate`.
    #[default]
    Auto,
    /// Tracker 3 only.
    Tracker,
    /// KDE's Baloo only.
    Baloo,
    /// No index: `fd` (else `plocate`) under the home folder, within the time budget.
    Fd,
    /// No file search.
    Off,
}

/// `launcher.clipboard_history`: whether copied things are kept for the launcher.
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
pub enum ClipboardHistory {
    /// Kept in memory only, forgotten when sill stops; password-manager copies are never kept.
    #[default]
    Memory,
    /// Nothing is kept, and sill does not watch the clipboard.
    Off,
}

/// `launcher.emoji_skin_tone`: the tone applied to emoji that take one.
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
pub enum EmojiSkinTone {
    /// The yellow default.
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

/// `launcher.web_search`: whether typed text offers "Search the web for …".
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
pub enum WebSearch {
    #[default]
    On,
    Off,
}

/// `launcher.currency`: currency conversion in the calculator, which needs daily rates from the
/// network.
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
pub enum CurrencyRates {
    /// No currencies; nothing is fetched.
    #[default]
    Off,
    /// The European Central Bank's daily reference rates, fetched at most once a day.
    Ecb,
}
