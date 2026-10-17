//! The identifiers a launcher result carries.

use serde::{Deserialize, Serialize};

/// Which provider produced a result. A closed set: Tab cycles through them in this order
/// (design/06-INTERACTIONS.md §20.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Installed applications (desktop entries).
    Apps,
    /// Open windows (toplevels).
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
    /// Things apps index (mail, contacts, events), found through intentd's shadow index.
    Things,
    /// The companion: the Ask row, the live answer and recent activity.
    Companion,
    /// The helpers the person keeps, and the row that starts a new one.
    Helpers,
    /// The computer-use runs now and just ended.
    Runs,
    /// What the companion remembers: the memory timeline.
    Memory,
}

impl ProviderKind {
    /// Every provider, in Tab order. Files, Clipboard, Emoji and Web follow the first five, so a
    /// tie between rows of equal score still goes to the older kinds first
    /// (`sill_model::results::step`); Things, Companion, Helpers, Runs and Memory come last.
    pub const ALL: [ProviderKind; 14] = [
        ProviderKind::Apps,
        ProviderKind::Windows,
        ProviderKind::Calculator,
        ProviderKind::Settings,
        ProviderKind::System,
        ProviderKind::Files,
        ProviderKind::Clipboard,
        ProviderKind::Emoji,
        ProviderKind::Web,
        ProviderKind::Things,
        ProviderKind::Companion,
        ProviderKind::Helpers,
        ProviderKind::Runs,
        ProviderKind::Memory,
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
