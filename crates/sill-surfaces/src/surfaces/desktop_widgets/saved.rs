//! Where the desktop widgets sit, kept across runs, per output:
//! `$XDG_STATE_HOME/sill/desktop-widgets.json` (state, not a setting, like the dock's pins), a
//! JSON array of `{"output", "kind", "column", "row"}`. `output` is the connector name
//! (`DP-1`); an entry without one (the file before drag, FINDINGS "Widget drag") is every
//! output's until that output has places of its own. The layer reads and writes it only through
//! a [`PlacementFile`] its root was given (the daemon provides one): a test or a render with none
//! keeps its placements in memory and never touches a file.
//!
//! The format is small and fixed, and sill-surfaces has no JSON crate (F400 adds none), so
//! [`encode`] writes it and [`decode`] reads it by hand; an entry it cannot read is skipped, a
//! file it cannot read is no placements.

use std::path::PathBuf;

use sill_settings::WidgetKind;

use super::placement::{GridSlot, Placement};

/// Which output an entry belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputKey {
    /// No output named: an entry from before places were per output, or an output with no name.
    Any,
    /// The output's connector name.
    Named(String),
}

impl OutputKey {
    /// The key for an output named `name`; a name the file cannot spell plainly (a quote, a
    /// backslash, a comma, a brace or a colon) is `Any`.
    pub fn of(name: Option<&str>) -> OutputKey {
        let plain = |name: &str| !name.is_empty() && !name.contains(UNSPELLABLE);
        match name {
            Some(name) if plain(name) => OutputKey::Named(name.to_owned()),
            Some(_) | None => OutputKey::Any,
        }
    }
}

/// What the hand-written format cannot carry in a name.
const UNSPELLABLE: [char; 6] = ['"', '\\', ',', '{', '}', ':'];

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

/// `output`'s placements: its own entries, or the entries with no output while it has none.
pub fn for_output(entries: &[SavedEntry], output: &OutputKey) -> Vec<Placement> {
    let of = |key: &OutputKey| -> Vec<Placement> {
        entries
            .iter()
            .filter(|e| e.output == *key)
            .map(|e| e.placement)
            .collect()
    };
    match of(output) {
        own if !own.is_empty() => own,
        _ => of(&OutputKey::Any),
    }
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

/// The kind's word in the file (the settings' spelling).
fn word(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Calendar => "calendar",
        WidgetKind::UpNext => "up_next",
        WidgetKind::Battery => "battery",
        WidgetKind::NowPlaying => "now_playing",
        WidgetKind::WorldClock => "world_clock",
    }
}

/// The kind a word names.
fn kind_of(word: &str) -> Option<WidgetKind> {
    [
        WidgetKind::Calendar,
        WidgetKind::UpNext,
        WidgetKind::Battery,
        WidgetKind::NowPlaying,
        WidgetKind::WorldClock,
    ]
    .into_iter()
    .find(|kind| self::word(*kind) == word)
}

/// `entries` as the file's JSON, one entry a line.
pub fn encode(entries: &[SavedEntry]) -> String {
    let lines: Vec<String> = entries.iter().map(line).collect();
    match lines.is_empty() {
        true => "[]\n".to_owned(),
        false => format!("[\n{}\n]\n", lines.join(",\n")),
    }
}

/// One entry's line.
fn line(entry: &SavedEntry) -> String {
    let output = match &entry.output {
        OutputKey::Named(name) => format!("\"output\": \"{name}\", "),
        OutputKey::Any => String::new(),
    };
    let p = entry.placement;
    format!(
        "  {{{output}\"kind\": \"{}\", \"column\": {}, \"row\": {}}}",
        word(p.kind),
        p.slot.column,
        p.slot.row
    )
}

/// The entries in the file's JSON: every `{...}` object with a known `kind` and a `column` and
/// `row` that are numbers, and an `output` when it has one; anything else is skipped.
pub fn decode(text: &str) -> Vec<SavedEntry> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| entry(rest.split('}').next()?))
        .collect()
}

/// One object's body: `"output": "DP-1", "kind": "battery", "column": 3, "row": 0`.
fn entry(body: &str) -> Option<SavedEntry> {
    let field = |name: &str| {
        body.split(',').find_map(|pair| {
            let (key, value) = pair.split_once(':')?;
            (key.trim().trim_matches('"') == name).then(|| value.trim().trim_matches('"'))
        })
    };
    let placement = Placement {
        kind: kind_of(field("kind")?)?,
        slot: GridSlot {
            column: field("column")?.parse().ok()?,
            row: field("row")?.parse().ok()?,
        },
    };
    Some(SavedEntry {
        output: OutputKey::of(field("output")),
        placement,
    })
}

#[cfg(test)]
#[path = "saved_tests.rs"]
mod tests;
