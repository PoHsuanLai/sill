//! The pins file: `dock.json` in the config directory, read at once and written atomically.

use std::path::Path;

use sill_model::pins::DockPins;

use crate::ServiceError;

/// The file's name inside `$XDG_CONFIG_HOME/sill`.
pub(crate) const FILE_NAME: &str = "dock.json";

/// The pins in `dir`; a missing or damaged file is no pins (the dock still shows running
/// apps).
pub(crate) fn load(dir: &Path) -> DockPins {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// Write `pins` to `dir` (created if needed) through a temporary file and a rename, so a
/// directory watch sees one rename and a crash leaves no half-written file.
pub(crate) fn save(dir: &Path, pins: &DockPins) -> Result<(), ServiceError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let path = dir.join(FILE_NAME);
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, render(pins)).map_err(io(&tmp))?;
    std::fs::rename(&tmp, &path).map_err(io(&path))
}

/// The file's contents; a damaged file is no pins (the dock still shows running apps).
fn parse(text: &str) -> DockPins {
    serde_json::from_str(text).unwrap_or_default()
}

/// The file's contents to write.
fn render(pins: &DockPins) -> String {
    // Strings only: nothing here can fail to encode.
    serde_json::to_string_pretty(pins).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{load, parse, render, save};
    use sill_launcher::AppId;
    use sill_model::pins::DockPins;
    use sill_testkit::TempDir;

    #[test]
    fn save_then_load_round_trips() {
        let dir = TempDir::new("pins-round-trip");
        let pins = DockPins {
            pinned: vec![
                AppId("org.mozilla.firefox".to_owned()),
                AppId("foot".to_owned()),
            ],
        };
        save(dir.path(), &pins).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(dir.path()), pins);
    }

    #[test]
    fn a_missing_or_damaged_file_loads_as_no_pins() {
        let dir = TempDir::new("pins-damaged");
        assert_eq!(load(dir.path()), DockPins::default(), "missing file");
        std::fs::write(dir.path().join(super::FILE_NAME), b"not json")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(dir.path()), DockPins::default(), "damaged file");
    }

    #[test]
    fn the_file_round_trips_and_damage_is_no_pins() {
        let p = DockPins {
            pinned: vec![
                AppId("org.mozilla.firefox".to_owned()),
                AppId("foot".to_owned()),
            ],
        };
        assert_eq!(parse(&render(&p)), p);
        assert_eq!(
            parse(r#"{"pinned":["foot"]}"#),
            DockPins {
                pinned: vec![AppId("foot".to_owned())],
            }
        );
        for text in ["", "[]", "{\"pinned\": 3}", "garbage"] {
            assert_eq!(parse(text), DockPins::default(), "{text:?}");
        }
    }
}
