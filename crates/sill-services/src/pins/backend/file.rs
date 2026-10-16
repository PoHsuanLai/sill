//! The pins file: `dock.json` in the config directory, read at once and written atomically.

use std::path::Path;

use sill_model::pins::DockPins;

use crate::ServiceError;
use crate::durable::{self, Access};

/// The file's name inside `$XDG_CONFIG_HOME/sill`.
pub(crate) const FILE_NAME: &str = "dock.json";

/// The pins in `dir`; a missing or damaged file is no pins (the dock still shows running
/// apps).
pub(crate) fn load(dir: &Path) -> DockPins {
    let path = dir.join(FILE_NAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return DockPins::default();
    };
    match serde_json::from_str(&text) {
        Ok(pins) => pins,
        // The next save would replace it: keep what the person had.
        Err(_) => {
            durable::set_aside(&path);
            DockPins::default()
        }
    }
}

/// Write `pins` to `dir` (created if needed) durably, so a directory watch sees one rename and a
/// crash leaves no half-written file ([`crate::durable`]). The file keeps the permissions it has:
/// a person may keep their dock in a shared dotfiles folder.
pub(crate) fn save(dir: &Path, pins: &DockPins) -> Result<(), ServiceError> {
    durable::write_file(
        &dir.join(FILE_NAME),
        render(pins).as_bytes(),
        Access::AsFound,
    )
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
        assert!(
            dir.path().join("dock.json.damaged").exists(),
            "the damaged file is kept for the person"
        );
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
