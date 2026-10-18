//! The launcher v2 keys' value types.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// `launcher.web_search`: whether typed text offers "Search the web for …".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum WebSearch {
    #[default]
    On,
    Off,
}
