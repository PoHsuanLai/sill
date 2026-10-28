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

    /// The provider's word in `frecency.json` keys (`apps:org.mozilla.firefox`): the serde name,
    /// kept for good, since the file outlives the build.
    pub fn slug(self) -> &'static str {
        match self {
            ProviderKind::Apps => "apps",
            ProviderKind::Windows => "windows",
            ProviderKind::Calculator => "calculator",
            ProviderKind::Settings => "settings",
            ProviderKind::System => "system",
            ProviderKind::Files => "files",
            ProviderKind::Clipboard => "clipboard",
            ProviderKind::Emoji => "emoji",
            ProviderKind::Web => "web",
            ProviderKind::Things => "things",
            ProviderKind::Companion => "companion",
            ProviderKind::Helpers => "helpers",
            ProviderKind::Runs => "runs",
            ProviderKind::Memory => "memory",
        }
    }

    /// Whether choosing the provider's rows is counted in `frecency.json`. The web fallback is
    /// counted under its fixed id (never the query, which is not a row's identity); a clipboard
    /// entry's key means nothing after the session, and the history must leave no trace of its
    /// use.
    pub fn counted(self) -> bool {
        match self {
            // An answer, a helper, a run and a memory row name something that ends or is
            // forgotten; choosing one is not a habit to rank by.
            ProviderKind::Clipboard
            | ProviderKind::Companion
            | ProviderKind::Helpers
            | ProviderKind::Runs
            | ProviderKind::Memory => false,
            // What a person calculated is in the row's id, and it must not be kept in a file; a
            // result is not a habit to rank by either.
            ProviderKind::Calculator => false,
            ProviderKind::Apps
            | ProviderKind::Windows
            | ProviderKind::Settings
            | ProviderKind::System
            | ProviderKind::Files
            | ProviderKind::Emoji
            | ProviderKind::Things
            | ProviderKind::Web => true,
        }
    }

    /// Whether the provider's rows are never Recent: System rows are Actions already, clipboard
    /// use is never counted, an answer, a helper, a run and a memory row are not habits, and the
    /// web row says nothing about what was searched.
    pub fn never_recent(self) -> bool {
        matches!(
            self,
            ProviderKind::System
                | ProviderKind::Clipboard
                | ProviderKind::Companion
                | ProviderKind::Helpers
                | ProviderKind::Runs
                | ProviderKind::Memory
                | ProviderKind::Web
        )
    }
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
