//! What the search hears and what it can be told.

use serde::{Deserialize, Serialize};
use sill_launcher::{Batch, Choice, Generation, ItemKey, ProviderKind};

use super::LauncherPolicy;

/// One step of the search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ResultsEvent {
    /// A keystroke began `generation`: older rows go.
    Began(Generation),
    /// A provider answered.
    Batch(Batch),
}

/// What the search can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCommand {
    /// The text changed: begin a generation and query every provider.
    Query(String),
    /// The text changed under the Tab filter: begin a generation and query `provider` alone
    /// (`Scope::Only`), so its rows are not capped by the other providers'.
    Filtered {
        text: String,
        provider: ProviderKind,
    },
    /// A row was chosen: count it for frecency.
    Chosen { key: ItemKey, choice: Choice },
    /// The launcher closed: cancel the running generation.
    Cancel,
    /// The settings changed what the settings-gated providers may do; the daemon sends it at start,
    /// and until then every one of them is off.
    Policy(LauncherPolicy),
}
