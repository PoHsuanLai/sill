//! The screen corners that can run an action.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// A screen corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
