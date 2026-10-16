//! `$XDG_STATE_HOME/sill/displays.json` on disk: where the record of seen displays lives, read
//! and written atomically.

use std::path::{Path, PathBuf};

use sill_model::displays::seen::SeenDisplays;

use crate::durable::{self, Access};
use crate::error::ServiceError;

/// `displays.json` in sill's folder of `dirs`' state directory.
pub fn path_in(dirs: &crate::env::Dirs) -> PathBuf {
    dirs.sill_state().join("displays.json")
}

/// The record at `path`; empty when there is none or it cannot be read (a broken record costs
/// at most one rescale per display, never a failed start).
pub fn load(path: &Path) -> SeenDisplays {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Write `seen` to `path` durably ([`crate::durable`]).
pub fn save(path: &Path, seen: &SeenDisplays) -> Result<(), ServiceError> {
    let text = serde_json::to_string_pretty(seen).map_err(|e| ServiceError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(e),
    })?;
    durable::write_file(path, text.as_bytes(), Access::OwnerOnly)
}

#[cfg(test)]
mod tests {
    use super::{load, save};
    use sill_model::displays::identity::IdentityKey;
    use sill_model::displays::scale::DisplayScale;
    use sill_model::displays::seen::{SeenDisplay, SeenDisplays};

    const S150: DisplayScale = DisplayScale(150);

    #[test]
    fn the_record_round_trips_and_a_missing_one_is_empty() {
        let dir = std::env::temp_dir().join(format!("sill-seen-{}", std::process::id()));
        let path = dir.join("sill").join("displays.json");
        assert_eq!(load(&path), SeenDisplays::default());
        let mut seen = SeenDisplays::default();
        seen.displays.insert(
            IdentityKey("BNQ-8074-00".to_owned()),
            SeenDisplay {
                name: "BenQ MA270U".to_owned(),
                applied: Some(S150),
            },
        );
        save(&path, &seen).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), seen);
        std::fs::write(&path, "not json").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), SeenDisplays::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
