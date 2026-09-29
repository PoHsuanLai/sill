//! Where the desktop widgets sit, kept across runs, per output:
//! `$XDG_STATE_HOME/sill/desktop-widgets.json` (state, not a setting, like the dock's pins), a
//! JSON array of `{"output", "kind", "column", "row"}`. `output` is the connector name (`DP-1`),
//! left out for an output with no name. The layer reads and writes it only through a
//! [`PlacementFile`] its root was given (the daemon provides one): a test or a render with none
//! keeps its placements in memory and never touches a file. An entry that cannot be read is
//! skipped, a file that cannot be read is no placements.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_settings::WidgetKind;

use super::placement::{GridSlot, Placement};

/// Which output an entry belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputKey {
    /// An output with no name.
    Unnamed,
    /// The output's connector name.
    Named(String),
}

impl OutputKey {
    /// The key for an output named `name`; no name, or an empty one, is `Unnamed`.
    pub fn of(name: Option<&str>) -> OutputKey {
        match name {
            Some(name) if !name.is_empty() => OutputKey::Named(name.to_owned()),
            Some(_) | None => OutputKey::Unnamed,
        }
    }
}

/// One entry of the file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SavedEntry {
    pub output: OutputKey,
    pub placement: Placement,
}

/// The file the placements are kept in, provided as root context by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlacementFile(pub PathBuf);

impl PlacementFile {
    /// `desktop-widgets.json` under `$XDG_STATE_HOME/sill`, else `$HOME/.local/state/sill`.
    pub fn default_path() -> Option<PlacementFile> {
        dirs::state_dir().map(|dir| PlacementFile(dir.join("sill").join("desktop-widgets.json")))
    }

    /// Every entry in the file; none when it is missing or unreadable.
    pub fn entries(&self) -> Vec<SavedEntry> {
        std::fs::read_to_string(&self.0)
            .map(|text| decode(&text))
            .unwrap_or_default()
    }

    /// `output`'s placements ([`for_output`]).
    pub fn load(&self, output: &OutputKey) -> Vec<Placement> {
        for_output(&self.entries(), output)
    }

    /// Make `placements` `output`'s, keeping every other output's, through a temporary file and
    /// a rename.
    pub fn save(&self, output: &OutputKey, placements: &[Placement]) -> std::io::Result<()> {
        let entries = with_output(self.entries(), output, placements);
        if let Some(dir) = self.0.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.0.with_extension("part");
        std::fs::write(&tmp, encode(&entries))?;
        std::fs::rename(&tmp, &self.0)
    }
}

/// `output`'s placements.
pub fn for_output(entries: &[SavedEntry], output: &OutputKey) -> Vec<Placement> {
    entries
        .iter()
        .filter(|e| e.output == *output)
        .map(|e| e.placement)
        .collect()
}

/// `entries` with `output`'s replaced by `placements` (after every other output's).
pub fn with_output(
    entries: Vec<SavedEntry>,
    output: &OutputKey,
    placements: &[Placement],
) -> Vec<SavedEntry> {
    entries
        .into_iter()
        .filter(|e| e.output != *output)
        .chain(placements.iter().map(|p| SavedEntry {
            output: output.clone(),
            placement: *p,
        }))
        .collect()
}

/// One entry as the file spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Wire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    kind: WidgetKind,
    column: u16,
    row: u16,
}

impl From<&SavedEntry> for Wire {
    fn from(entry: &SavedEntry) -> Wire {
        Wire {
            output: match &entry.output {
                OutputKey::Named(name) => Some(name.clone()),
                OutputKey::Unnamed => None,
            },
            kind: entry.placement.kind,
            column: entry.placement.slot.column,
            row: entry.placement.slot.row,
        }
    }
}

impl From<Wire> for SavedEntry {
    fn from(wire: Wire) -> SavedEntry {
        SavedEntry {
            output: OutputKey::of(wire.output.as_deref()),
            placement: Placement {
                kind: wire.kind,
                slot: GridSlot {
                    column: wire.column,
                    row: wire.row,
                },
            },
        }
    }
}

/// `entries` as the file's JSON.
pub fn encode(entries: &[SavedEntry]) -> String {
    let wires: Vec<Wire> = entries.iter().map(Wire::from).collect();
    // Strings, a closed enum and numbers: nothing here can fail to encode.
    serde_json::to_string_pretty(&wires).unwrap_or_default() + "\n"
}

/// The entries in the file's JSON array; an element that is not an entry is skipped.
pub fn decode(text: &str) -> Vec<SavedEntry> {
    serde_json::from_str::<Vec<serde_json::Value>>(text)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Wire>(value).ok())
        .map(SavedEntry::from)
        .collect()
}

#[cfg(test)]
#[path = "saved_tests.rs"]
mod tests;
