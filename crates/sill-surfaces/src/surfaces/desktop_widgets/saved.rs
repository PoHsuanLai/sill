//! Where the desktop widgets sit, kept across runs: `$XDG_STATE_HOME/sill/desktop-widgets.json`
//! (state, not a setting, like the dock's pins), a JSON array of `{"kind", "column", "row"}`.
//! The layer reads and writes it only through a [`PlacementFile`] its root was given (the
//! daemon provides one): a test or a render with none keeps its placements in memory and never
//! touches a file.
//!
//! The format is small and fixed, and sill-surfaces has no JSON crate (F400 adds none), so
//! [`encode`] writes it and [`decode`] reads it by hand; an entry it cannot read is skipped, a
//! file it cannot read is no placements.

use std::path::PathBuf;

use sill_settings::WidgetKind;

use super::placement::{GridSlot, Placement};

/// The file the placements are kept in, provided as root context by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlacementFile(pub PathBuf);

impl PlacementFile {
    /// `desktop-widgets.json` under `$XDG_STATE_HOME/sill`, else `$HOME/.local/state/sill`.
    pub fn default_path() -> Option<PlacementFile> {
        dirs::state_dir().map(|dir| PlacementFile(dir.join("sill").join("desktop-widgets.json")))
    }

    /// The placements in the file; none when it is missing or unreadable.
    pub fn load(&self) -> Vec<Placement> {
        std::fs::read_to_string(&self.0)
            .map(|text| decode(&text))
            .unwrap_or_default()
    }

    /// Write `placements` through a temporary file and a rename.
    pub fn save(&self, placements: &[Placement]) -> std::io::Result<()> {
        if let Some(dir) = self.0.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.0.with_extension("part");
        std::fs::write(&tmp, encode(placements))?;
        std::fs::rename(&tmp, &self.0)
    }
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

/// `placements` as the file's JSON, one entry a line.
pub fn encode(placements: &[Placement]) -> String {
    let entries: Vec<String> = placements
        .iter()
        .map(|p| {
            format!(
                "  {{\"kind\": \"{}\", \"column\": {}, \"row\": {}}}",
                word(p.kind),
                p.slot.column,
                p.slot.row
            )
        })
        .collect();
    match entries.is_empty() {
        true => "[]\n".to_owned(),
        false => format!("[\n{}\n]\n", entries.join(",\n")),
    }
}

/// The placements in the file's JSON: every `{...}` object with a known `kind` and a `column`
/// and `row` that are numbers; anything else is skipped.
pub fn decode(text: &str) -> Vec<Placement> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| entry(rest.split('}').next()?))
        .collect()
}

/// One object's body: `"kind": "battery", "column": 3, "row": 0`.
fn entry(body: &str) -> Option<Placement> {
    let field = |name: &str| {
        body.split(',').find_map(|pair| {
            let (key, value) = pair.split_once(':')?;
            (key.trim().trim_matches('"') == name).then(|| value.trim().trim_matches('"'))
        })
    };
    Some(Placement {
        kind: kind_of(field("kind")?)?,
        slot: GridSlot {
            column: field("column")?.parse().ok()?,
            row: field("row")?.parse().ok()?,
        },
    })
}

#[cfg(test)]
mod tests {
    use sill_settings::WidgetKind;

    use super::{PlacementFile, decode, encode};
    use crate::surfaces::desktop_widgets::placement::{GridSlot, Placement};

    fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
        Placement {
            kind,
            slot: GridSlot { column, row },
        }
    }

    #[test]
    fn placements_round_trip_through_the_file_format() {
        let placements = vec![
            at(WidgetKind::Calendar, 9, 0),
            at(WidgetKind::UpNext, 7, 1),
            at(WidgetKind::NowPlaying, 0, 4),
            at(WidgetKind::WorldClock, 2, 2),
            at(WidgetKind::Battery, 9, 1),
        ];
        let text = encode(&placements);
        assert!(
            text.contains(r#"{"kind": "up_next", "column": 7, "row": 1}"#),
            "{text}"
        );
        assert_eq!(decode(&text), placements);
        assert_eq!(encode(&[]), "[]\n");
        assert_eq!(decode("[]"), Vec::new());
    }

    #[test]
    fn what_cannot_be_read_is_skipped() {
        let cases = [
            ("", Vec::new()),
            ("not json", Vec::new()),
            (
                r#"[{"kind":"weather","column":1,"row":1},{"row":2,"column":3,"kind":"battery"}]"#,
                vec![at(WidgetKind::Battery, 3, 2)],
            ),
            (
                r#"[{"kind":"calendar","column":-1,"row":0},{"kind":"calendar","column":1}]"#,
                Vec::new(),
            ),
        ];
        for (text, want) in cases {
            assert_eq!(decode(text), want, "{text}");
        }
    }

    #[test]
    fn the_file_is_written_and_read_back() {
        let dir = std::env::temp_dir().join(format!("sill-dw-saved-{}", std::process::id()));
        let file = PlacementFile(dir.join("state").join("desktop-widgets.json"));
        assert_eq!(file.load(), Vec::new(), "missing is none");
        let placements = vec![at(WidgetKind::Battery, 4, 0)];
        file.save(&placements)
            .unwrap_or_else(|e| panic!("saving: {e}"));
        assert_eq!(file.load(), placements);
        let _ = std::fs::remove_dir_all(dir);
    }
}
