//! The pins file: the `[[dock]]` rows of `layout.toml` in the config directory (design/28 §5.6),
//! read at once and written atomically. The same file will hold the shell's other placements, so
//! a save keeps every table it does not own. A dock the person had in the old `dock.json` is
//! carried over once, when `layout.toml` has no dock yet.

use std::path::Path;

use sill_launcher::AppId;
use sill_model::pins::DockPins;
use sill_model::seed::Store;
use toml::{Table, Value};

use super::legacy;
use crate::ServiceError;
use crate::durable::{self, Access};

/// The file's name inside `$XDG_CONFIG_HOME/sill`.
pub(crate) const FILE_NAME: &str = "layout.toml";

/// The file format's version.
const VERSION: i64 = 1;

/// The kind of a row that places an app in the dock.
const APP_KIND: &str = "quire.dock.app";

/// The pins in `dir`: those of `layout.toml`, else those of an old `dock.json` (written to
/// `layout.toml` at once, with a line on stderr), else none (the dock still shows running apps).
/// A `layout.toml` that is not TOML is kept as `.damaged` and counts as having no dock.
pub(crate) fn load(dir: &Path) -> DockPins {
    if let Some(pins) = dock_of(&read_table(dir)) {
        return pins;
    }
    let Some(pins) = legacy::read(dir) else {
        return DockPins::default();
    };
    match save(dir, &pins) {
        Ok(()) => eprintln!(
            "sill: the dock moved from {} to {} ({} is left as it was)",
            legacy::FILE_NAME,
            FILE_NAME,
            legacy::FILE_NAME
        ),
        Err(error) => eprintln!("sill: saving {}: {error}", dir.display()),
    }
    pins
}

/// Whether the dock was ever written: `layout.toml` has a dock, even an emptied one.
pub(crate) fn store(dir: &Path) -> Store {
    match dock_of(&read_table_quietly(dir)) {
        Some(_) => Store::Present,
        None => Store::Missing,
    }
}

/// Write `pins` to `dir` (created if needed) durably, so a directory watch sees one rename and a
/// crash leaves no half-written file ([`crate::durable`]). The file keeps the permissions it has:
/// a person may keep their layout in a shared dotfiles folder. Tables of other placements stay.
pub(crate) fn save(dir: &Path, pins: &DockPins) -> Result<(), ServiceError> {
    durable::write_file(
        &dir.join(FILE_NAME),
        render(read_table(dir), pins).as_bytes(),
        Access::AsFound,
    )
}

/// The file's tables; a missing file has none, a file that is not TOML is set aside.
fn read_table(dir: &Path) -> Table {
    let path = dir.join(FILE_NAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Table::new();
    };
    text.parse().unwrap_or_else(|_| {
        // The next save would replace it: keep what the person had.
        durable::set_aside(&path);
        Table::new()
    })
}

/// As [`read_table`], leaving a damaged file where it is (a question is not a reason to move it).
fn read_table_quietly(dir: &Path) -> Table {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default()
}

/// The pins the `[[dock]]` rows of `table` place, in order; `None` when it has no dock. A row of
/// another kind or without an app is not one of ours: it is left out here and kept by a save.
fn dock_of(table: &Table) -> Option<DockPins> {
    let rows = table.get("dock")?.as_array()?;
    let pinned = rows
        .iter()
        .filter_map(Value::as_table)
        .filter(|row| row.get("kind").and_then(Value::as_str) == Some(APP_KIND))
        .filter_map(|row| row.get("app").and_then(Value::as_str))
        .map(|app| AppId(app.to_owned()))
        .collect();
    Some(DockPins { pinned })
}

/// Whether `row` is one this file's pins come from: an app row that names its app.
fn is_ours(row: &Value) -> bool {
    row.as_table().is_some_and(|row| {
        row.get("kind").and_then(Value::as_str) == Some(APP_KIND)
            && row.get("app").and_then(Value::as_str).is_some()
    })
}

/// The contents to write: `table` with the version and the dock rows of `pins`. Rows of other
/// kinds (a later version's stacks, say) are kept, after the apps, in the order they had.
fn render(mut table: Table, pins: &DockPins) -> String {
    let kept: Vec<Value> = table
        .get("dock")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter(|row| !is_ours(row)).cloned().collect())
        .unwrap_or_default();
    let rows = pins
        .pinned
        .iter()
        .map(|app| {
            let mut row = Table::new();
            row.insert("kind".to_owned(), Value::String(APP_KIND.to_owned()));
            row.insert("app".to_owned(), Value::String(app.0.clone()));
            Value::Table(row)
        })
        .chain(kept)
        .collect();
    table.insert("version".to_owned(), Value::Integer(VERSION));
    table.insert("dock".to_owned(), Value::Array(rows));
    // Strings and integers only: nothing here can fail to encode.
    toml::to_string(&table).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{FILE_NAME, load, save, store};
    use sill_launcher::AppId;
    use sill_model::pins::DockPins;
    use sill_model::seed::Store;
    use sill_testkit::TempDir;

    fn pins(ids: &[&str]) -> DockPins {
        DockPins {
            pinned: ids.iter().map(|id| AppId((*id).to_owned())).collect(),
        }
    }

    fn write(dir: &TempDir, name: &str, text: &str) {
        std::fs::write(dir.path().join(name), text).unwrap_or_else(|e| panic!("{e}"));
    }

    fn read(dir: &TempDir, name: &str) -> String {
        std::fs::read_to_string(dir.path().join(name)).unwrap_or_default()
    }

    #[test]
    fn save_then_load_round_trips_in_order() {
        let dir = TempDir::new("pins-round-trip");
        let want = pins(&["org.mozilla.firefox", "foot", "btop"]);
        save(dir.path(), &want).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(dir.path()), want);
        let text = read(&dir, FILE_NAME);
        assert!(text.contains("version = 1"), "{text}");
        assert!(text.contains("[[dock]]"), "{text}");
        assert_eq!(store(dir.path()), Store::Present);
    }

    #[test]
    fn a_save_keeps_the_tables_it_does_not_own() {
        let dir = TempDir::new("pins-keeps-others");
        write(
            &dir,
            FILE_NAME,
            "[[center_widgets]]\nkind = \"quire.battery\"\n",
        );
        save(dir.path(), &pins(&["foot"])).unwrap_or_else(|e| panic!("{e}"));
        let text = read(&dir, FILE_NAME);
        assert!(text.contains("center_widgets"), "{text}");
        assert_eq!(load(dir.path()), pins(&["foot"]));
    }

    #[test]
    fn the_old_dock_is_carried_over_once_and_left_in_place() {
        let dir = TempDir::new("pins-migrate");
        let old = r#"{"pinned":["foot","btop"]}"#;
        write(&dir, "dock.json", old);
        assert_eq!(store(dir.path()), Store::Missing);
        assert_eq!(load(dir.path()), pins(&["foot", "btop"]));
        assert_eq!(store(dir.path()), Store::Present);
        assert_eq!(read(&dir, "dock.json"), old, "the old file is untouched");
        // The old file changes later: layout.toml has the dock, so it is not read again.
        write(&dir, "dock.json", r#"{"pinned":["other"]}"#);
        assert_eq!(load(dir.path()), pins(&["foot", "btop"]));
    }

    #[test]
    fn an_existing_dock_in_layout_is_never_replaced_by_the_old_file() {
        let dir = TempDir::new("pins-no-migrate");
        save(dir.path(), &pins(&["mine"])).unwrap_or_else(|e| panic!("{e}"));
        write(&dir, "dock.json", r#"{"pinned":["old"]}"#);
        assert_eq!(load(dir.path()), pins(&["mine"]));
        // An emptied dock is a dock too.
        save(dir.path(), &pins(&[])).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(dir.path()), pins(&[]));
        assert_eq!(store(dir.path()), Store::Present);
    }

    #[test]
    fn a_missing_file_loads_as_no_pins_and_writes_nothing() {
        let dir = TempDir::new("pins-missing");
        assert_eq!(load(dir.path()), DockPins::default());
        assert!(!dir.path().join(FILE_NAME).exists());
        assert_eq!(store(dir.path()), Store::Missing);
    }

    #[test]
    fn a_damaged_layout_is_kept_and_the_old_dock_still_carries_over() {
        let dir = TempDir::new("pins-damaged-layout");
        write(&dir, FILE_NAME, "not = [toml");
        write(&dir, "dock.json", r#"{"pinned":["foot"]}"#);
        assert_eq!(load(dir.path()), pins(&["foot"]));
        assert_eq!(read(&dir, "layout.toml.damaged"), "not = [toml");
        assert_eq!(read(&dir, "dock.json"), r#"{"pinned":["foot"]}"#);
    }

    #[test]
    fn a_damaged_old_file_is_no_pins_and_stays_where_it_is() {
        let dir = TempDir::new("pins-damaged-old");
        for text in ["", "{\"pinned\": 3}", "garbage"] {
            write(&dir, "dock.json", text);
            assert_eq!(load(dir.path()), DockPins::default(), "{text:?}");
            assert!(!dir.path().join(FILE_NAME).exists(), "{text:?}");
            assert_eq!(read(&dir, "dock.json"), text);
        }
    }

    #[test]
    fn rows_of_another_kind_or_without_an_app_are_left_out() {
        let dir = TempDir::new("pins-lenient-rows");
        write(
            &dir,
            FILE_NAME,
            "version = 1\n[[dock]]\nkind = \"quire.dock.app\"\napp = \"a\"\n\
             [[dock]]\nkind = \"quire.dock.stack\"\npath = \"/x\"\n\
             [[dock]]\nkind = \"quire.dock.app\"\n\
             [[dock]]\nkind = \"quire.dock.app\"\napp = \"b\"\n",
        );
        assert_eq!(load(dir.path()), pins(&["a", "b"]));
        // A save rewrites the apps and keeps the rows that are not ours, in their order.
        save(dir.path(), &pins(&["b"])).unwrap_or_else(|e| panic!("{e}"));
        let text = read(&dir, FILE_NAME);
        assert_eq!(load(dir.path()), pins(&["b"]));
        assert!(text.contains("quire.dock.stack"), "{text}");
        assert!(text.contains("path = \"/x\""), "{text}");
    }
}
