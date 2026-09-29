//! The identifiers a launcher result carries.

use serde::{Deserialize, Serialize};

/// Which provider produced a result. A closed set: Tab cycles through them in this order
/// (design/06-INTERACTIONS.md §20.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Installed applications (desktop entries).
    Apps,
    /// Open windows (COSMIC toplevels).
    Windows,
    /// Arithmetic and unit conversion.
    Calculator,
    /// Settings pages and keys.
    Settings,
    /// Lock, log out, suspend, restart, shut down, reload.
    System,
    /// Documents and folders under the home directory, from a file index.
    Files,
    /// The clipboard's history, newest first.
    Clipboard,
    /// Emoji by name, keyword and CLDR short name.
    Emoji,
    /// "Search the web for …": the fallback row.
    Web,
}

impl ProviderKind {
    /// Every provider, in Tab order. Files, Clipboard, Emoji and Web follow the first five, so a
    /// tie between rows of equal score still goes to the older kinds first
    /// (`sill_model::results::step`).
    pub const ALL: [ProviderKind; 9] = [
        ProviderKind::Apps,
        ProviderKind::Windows,
        ProviderKind::Calculator,
        ProviderKind::Settings,
        ProviderKind::System,
        ProviderKind::Files,
        ProviderKind::Clipboard,
        ProviderKind::Emoji,
        ProviderKind::Web,
    ];
}

/// A desktop entry id without `.desktop`: `org.mozilla.firefox`. The key the dock, the app
/// index and the launcher agree on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(pub String);

/// A result's identity within its provider, stable across keystrokes so selection can follow
/// it and frecency can count it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemId(pub String);

/// One of a result's secondary actions (Ctrl+K / Alt+K), e.g. a desktop action id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionId(pub String);

/// One clipboard history entry, by the number the clipboard service gave it when it was copied.
/// Never reused within a session; carries no content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClipKey(pub u64);
