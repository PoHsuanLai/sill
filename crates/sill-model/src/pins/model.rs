//! The pins' state.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use crate::apps::AppIndex;

/// The `[[dock]]` rows of `layout.toml`: the pinned apps, left to right.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DockPins {
    #[serde(default)]
    pub pinned: Vec<AppId>,
}

impl DockPins {
    /// The pins with each id of an app that was renamed (`X-Flatpak-RenamedFrom`) replaced by
    /// its current id. An app pinned under both ids is pinned once, where the first of them was.
    pub fn renamed(self, index: &AppIndex) -> DockPins {
        let renames = index.renames();
        let mut pinned: Vec<AppId> = Vec::with_capacity(self.pinned.len());
        for id in self.pinned {
            let id = renames.of(&id);
            if !pinned.contains(&id) {
                pinned.push(id);
            }
        }
        DockPins { pinned }
    }
}
