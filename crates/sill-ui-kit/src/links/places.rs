//! What each output's desktop keeps across runs: which widgets it shows (its own list, once an
//! Edit Widgets on that output has made one) and where they sit. One file,
//! `$XDG_STATE_HOME/sill/desktop-widgets.json` (state, not a setting, like the dock's pins):
//! `{"outputs":[{"output":"DP-1","widgets":["calendar"],"places":[{"kind":"calendar","column":9,
//! "row":0}]}]}`. `output` is the connector name (`DP-1`), left out for an output with no name;
//! `widgets` is left out for an output that follows `widgets.desktop_widgets`. The layer reads and
//! writes it only through a [`PlacementFile`] its root was given (the daemon provides one): a
//! test or a render with none keeps its placements in memory and never touches a file. An entry
//! that cannot be read is skipped, a file that cannot be read is nothing saved.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sill_services::durable::{self, Access};
use sill_settings::WidgetKind;

use crate::widgets::grid::{GridSlot, OutputKey, Placement};

/// What one output keeps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SavedOutput {
    pub output: OutputKey,
    /// The widgets this output shows; none: it follows `widgets.desktop_widgets`.
    pub kinds: Option<Vec<WidgetKind>>,
    /// Where its widgets sit, including those it keeps a place for while they are away.
    pub places: Vec<Placement>,
}

/// The file the state is kept in, provided as root context by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlacementFile(pub PathBuf);

impl PlacementFile {
    /// `desktop-widgets.json` in sill's folder of `dirs`' state directory.
    pub fn in_dirs(dirs: &sill_services::env::Dirs) -> PlacementFile {
        PlacementFile(dirs.sill_state().join("desktop-widgets.json"))
    }

    /// Every output in the file; none when it is missing or unreadable.
    pub fn outputs(&self) -> Vec<SavedOutput> {
        std::fs::read_to_string(&self.0)
            .map(|text| decode(&text))
            .unwrap_or_default()
    }

    /// `output`'s placements ([`places_of`]).
    pub fn load(&self, output: &OutputKey) -> Vec<Placement> {
        places_of(&self.outputs(), output)
    }

    /// The widgets `output` shows, when it has a list of its own ([`kinds_of`]).
    pub fn kinds(&self, output: &OutputKey) -> Option<Vec<WidgetKind>> {
        kinds_of(&self.outputs(), output)
    }

    /// Make `places` `output`'s, keeping its widget list and every other output's.
    pub fn save(&self, output: &OutputKey, places: &[Placement]) -> std::io::Result<()> {
        let kinds = self.kinds(output);
        self.write(output, kinds, places)
    }

    /// Make `kinds` the widgets `output` shows and `places` where they sit, keeping every other
    /// output's.
    pub fn save_layout(
        &self,
        output: &OutputKey,
        kinds: &[WidgetKind],
        places: &[Placement],
    ) -> std::io::Result<()> {
        self.write(output, Some(kinds.to_vec()), places)
    }

    /// Replace `output`'s record, durably ([`sill_services::durable`]). A file that cannot be
    /// read is kept aside first, so merging over it does not destroy what the person had.
    fn write(
        &self,
        output: &OutputKey,
        kinds: Option<Vec<WidgetKind>>,
        places: &[Placement],
    ) -> std::io::Result<()> {
        let record = SavedOutput {
            output: output.clone(),
            kinds,
            places: places.to_vec(),
        };
        if let Ok(text) = std::fs::read_to_string(&self.0)
            && serde_json::from_str::<Value>(&text).is_err()
        {
            durable::set_aside(&self.0);
        }
        let outputs = with_output(self.outputs(), record);
        durable::write(&self.0, encode(&outputs).as_bytes(), Access::OwnerOnly)
    }
}

/// `output`'s placements; none for an output the file does not know.
pub fn places_of(outputs: &[SavedOutput], output: &OutputKey) -> Vec<Placement> {
    outputs
        .iter()
        .find(|saved| saved.output == *output)
        .map(|saved| saved.places.clone())
        .unwrap_or_default()
}

/// The widgets `output` shows, when it has a list of its own.
pub fn kinds_of(outputs: &[SavedOutput], output: &OutputKey) -> Option<Vec<WidgetKind>> {
    outputs
        .iter()
        .find(|saved| saved.output == *output)
        .and_then(|saved| saved.kinds.clone())
}

/// `outputs` with `record` in place of its output's (after every other output's).
pub fn with_output(outputs: Vec<SavedOutput>, record: SavedOutput) -> Vec<SavedOutput> {
    let others = outputs
        .into_iter()
        .filter(|saved| saved.output != record.output);
    others.chain(std::iter::once(record)).collect()
}

/// One placed widget as the file spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WirePlace {
    kind: WidgetKind,
    column: u16,
    row: u16,
}

/// One output as the file spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WireOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    widgets: Option<Vec<WidgetKind>>,
    places: Vec<WirePlace>,
}

impl From<&SavedOutput> for WireOutput {
    fn from(saved: &SavedOutput) -> WireOutput {
        WireOutput {
            output: match &saved.output {
                OutputKey::Named(name) => Some(name.clone()),
                OutputKey::Unnamed => None,
            },
            widgets: saved.kinds.clone(),
            places: saved
                .places
                .iter()
                .map(|p| WirePlace {
                    kind: p.kind,
                    column: p.slot.column,
                    row: p.slot.row,
                })
                .collect(),
        }
    }
}

/// `outputs` as the file's JSON.
pub fn encode(outputs: &[SavedOutput]) -> String {
    let wires: Vec<WireOutput> = outputs.iter().map(WireOutput::from).collect();
    let file = serde_json::json!({ "outputs": wires });
    // Strings, a closed enum and numbers: nothing here can fail to encode.
    serde_json::to_string_pretty(&file).unwrap_or_default() + "\n"
}

/// The outputs in the file's JSON; an element that is not one is skipped.
pub fn decode(text: &str) -> Vec<SavedOutput> {
    serde_json::from_str::<Value>(text)
        .ok()
        .as_ref()
        .and_then(|file| file.get("outputs"))
        .and_then(Value::as_array)
        .map(|outputs| outputs.iter().filter_map(output_of).collect())
        .unwrap_or_default()
}

/// One element of `outputs`: its places and its widget list each skip what cannot be read.
fn output_of(value: &Value) -> Option<SavedOutput> {
    let record = value.as_object()?;
    let each = |key: &str| record.get(key).and_then(Value::as_array);
    Some(SavedOutput {
        output: OutputKey::of(record.get("output").and_then(Value::as_str)),
        kinds: each("widgets").map(|kinds| {
            kinds
                .iter()
                .filter_map(|kind| serde_json::from_value(kind.clone()).ok())
                .collect()
        }),
        places: each("places")
            .map(|places| place_list(places))
            .unwrap_or_default(),
    })
}

/// The places among `values`.
fn place_list(values: &[Value]) -> Vec<Placement> {
    values
        .iter()
        .filter_map(|value| serde_json::from_value::<WirePlace>(value.clone()).ok())
        .map(|wire| Placement {
            kind: wire.kind,
            slot: GridSlot {
                column: wire.column,
                row: wire.row,
            },
        })
        .collect()
}

#[cfg(test)]
#[path = "places_tests.rs"]
mod tests;
