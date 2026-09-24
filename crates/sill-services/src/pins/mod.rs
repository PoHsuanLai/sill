//! The dock's pinned apps, `~/.config/sill/dock.json` (design/10 §10.6: "pinned items stay in
//! `~/.config/sill/dock.json`"; state, not a setting, design/22 §1 rule 3).

mod fs_watch;
mod model;
mod run;
#[cfg(test)]
mod test_support;

pub use model::{DockPins, PinAt, PinEvent, reduce_pins};

use std::path::{Path, PathBuf};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::error::ServiceError;

/// The file's name inside `$XDG_CONFIG_HOME/sill`.
pub const FILE_NAME: &str = "dock.json";

/// Load the pins, then apply every [`PinEvent`] sent to the control, writing the file
/// atomically after each; an external edit (another process, a text editor) is picked up
/// through a 30 ms debounced directory watch and published as if it were a `Replaced` event.
pub fn spawn(rt: &Handle) -> (watch::Receiver<DockPins>, Control<PinEvent>) {
    let dir = pins_dir();
    let initial = dir.as_deref().map(load).unwrap_or_default();
    let (tx, rx) = watch::channel(initial);
    let (control, commands) = Control::channel();
    rt.spawn(run::run(dir, tx, commands));
    (rx, control)
}

/// `$XDG_CONFIG_HOME/sill`, else `$HOME/.config/sill`; `None` when neither is set, in which
/// case [`spawn`] still applies commands, just without persistence or a watch.
fn pins_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("sill"))
}

/// The pins in `dir`; a missing or damaged file is no pins (the dock still shows running
/// apps).
pub fn load(dir: &Path) -> DockPins {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .map(|text| DockPins::from_json(&text))
        .unwrap_or_default()
}

/// Write `pins` to `dir` (created if needed) through a temporary file and a rename, so a
/// directory watch sees one rename and a crash leaves no half-written file.
pub fn save(dir: &Path, pins: &DockPins) -> Result<(), ServiceError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let path = dir.join(FILE_NAME);
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, pins.to_json()).map_err(io(&tmp))?;
    std::fs::rename(&tmp, &path).map_err(io(&path))
}

#[cfg(test)]
mod tests {
    use super::{DockPins, load, save};
    use crate::pins::test_support::TempDir;
    use sill_launcher::AppId;

    #[test]
    fn save_then_load_round_trips() {
        let dir = TempDir::new("pins-round-trip");
        let pins = DockPins {
            version: 1,
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
}
