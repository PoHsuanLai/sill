//! What the on-screen indicator can show.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// What the indicator shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum OsdKind {
    Volume,
    Brightness,
}
