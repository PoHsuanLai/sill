//! The pins' old home, `dock.json`: read once to carry the dock over to `layout.toml`, never
//! written and never removed (it is the person's file).

use std::path::Path;

use sill_model::pins::DockPins;

/// The old file's name inside `$XDG_CONFIG_HOME/sill`.
pub(super) const FILE_NAME: &str = "dock.json";

/// The pins the old file holds; a missing or unreadable file holds none to carry over.
pub(super) fn read(dir: &Path) -> Option<DockPins> {
    let text = std::fs::read_to_string(dir.join(FILE_NAME)).ok()?;
    serde_json::from_str(&text).ok()
}
