//! What the settings-gated providers (files, web, emoji, currency) are allowed to do, from the
//! settings. The daemon sends it with `SearchCommand::Policy` at start and on every change; until
//! then everything is off, so the daemon's `start` in a test never reads a file index, never offers a
//! web search and never converts a currency.

use std::time::Duration;

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_launcher::SkinTone;

/// Which file index the files provider asks (`launcher.files_backend`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FilesChoice {
    /// Tracker 3 when its miner answers on the session bus, else `fd` (or `plocate`) under the
    /// home directory.
    Auto,
    Tracker,
    /// `fd`, else `plocate`, under the home directory, within the time budget.
    #[serde(rename = "fd")]
    #[word(slug = "fd", label = "Fd")]
    Walk,
    /// No file search.
    #[default]
    Off,
}

/// The web fallback row (`launcher.web_search`, `launcher.web_engine_url`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WebPolicy {
    #[default]
    Off,
    /// Offer "Search the web for …" opening `template` with `{query}` replaced.
    On { template: String },
}

/// Currency conversion in the calculator (`launcher.currency`). Off: no rates are fetched and
/// currency units are not offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum CurrencyPolicy {
    #[default]
    Off,
    /// The European Central Bank's daily reference rates, fetched at most once a day.
    Ecb,
}

/// The settings-gated providers' policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LauncherPolicy {
    pub files: FilesChoice,
    /// How long one file search may take before its rows are dropped (the walking backends
    /// stop there; an index is asked once and waited on at most this long).
    pub files_budget: Duration,
    pub web: WebPolicy,
    pub emoji_tone: SkinTone,
    pub currency: CurrencyPolicy,
}
