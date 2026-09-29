//! The pins' state.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

/// `dock.json`: the pinned apps, left to right.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DockPins {
    #[serde(default)]
    pub pinned: Vec<AppId>,
}
