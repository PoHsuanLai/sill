//! Reading and writing `frecency.json` (effects: the filesystem).

use std::path::Path;

use sill_launcher::Frecency;

use crate::error::ServiceError;

/// The file's name inside `$XDG_STATE_HOME/sill`.
pub const FILE_NAME: &str = "frecency.json";

/// The history in `dir`; missing or damaged is empty.
pub fn load_frecency(dir: &Path) -> Frecency {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .map(|text| Frecency::from_json(&text))
        .unwrap_or_default()
}

/// Write `frecency` to `dir` (created if needed) by temp file and rename, so a crash never
/// leaves half a file.
pub fn save_frecency(dir: &Path, frecency: &Frecency) -> Result<(), ServiceError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let path = dir.join(FILE_NAME);
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, frecency.to_json()).map_err(io(&tmp))?;
    std::fs::rename(&tmp, &path).map_err(io(&path))
}

#[cfg(test)]
mod tests {
    use super::{FILE_NAME, load_frecency, save_frecency};
    use sill_testkit::TempDir;
    use sill_launcher::{Frecency, ItemId, ItemKey, ProviderKind, UnixTime};

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn key(provider: ProviderKind, id: &str) -> ItemKey {
        ItemKey::of(provider, &ItemId(id.to_owned()))
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temp_file() {
        let dir = TempDir::new("frecency-round-trip");
        let frecency = Frecency::default()
            .record(key(ProviderKind::Apps, "foot"), NOW)
            .record(key(ProviderKind::Apps, "foot"), NOW)
            .record(
                key(ProviderKind::System, "lock"),
                UnixTime(NOW.0 - 86_400 * 20),
            );
        let nested = dir.path().join("state/sill");
        save_frecency(&nested, &frecency).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load_frecency(&nested), frecency);
        let names: Vec<_> = std::fs::read_dir(&nested)
            .unwrap_or_else(|e| panic!("{e}"))
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from(FILE_NAME)]);
    }

    #[test]
    fn a_missing_or_damaged_file_is_an_empty_history() {
        let dir = TempDir::new("frecency-damaged");
        assert_eq!(load_frecency(dir.path()), Frecency::default(), "missing");
        for text in ["", "not json", r#"{"entries":{"apps:x":{"count":-1}}}"#] {
            std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(load_frecency(dir.path()), Frecency::default(), "{text:?}");
        }
    }

    #[test]
    fn a_file_written_by_hand_keeps_its_counts() {
        let dir = TempDir::new("frecency-by-hand");
        let text = r#"{"entries":{"apps:foot":{"count":3,"last":1790000000},"x":{"count":1,"last":0}},"later":1}"#;
        std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
        let loaded = load_frecency(dir.path());
        assert_eq!(
            loaded.entries.len(),
            2,
            "unknown top-level keys are ignored"
        );
        assert_eq!(
            loaded.score(&key(ProviderKind::Apps, "foot"), NOW),
            sill_launcher::FrecencyScore(300)
        );
    }
}
