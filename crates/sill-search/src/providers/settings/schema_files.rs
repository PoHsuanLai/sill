//! Which schema files are installed and whether they changed: the directory listing and the
//! cheap fingerprint the provider's cache compares. Reading a file is
//! `ds_settings::schema::Schema::from_toml`.

use std::fs;
use std::path::PathBuf;

/// Every `*.settings.toml` file directly under each of `dirs`, in directory order and then file
/// name order within a directory, the order `ds_settings::schema::discover` reads them in. A
/// directory that does not exist is skipped silently.
pub fn schema_paths(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with(".settings.toml"))
            })
            .collect();
        paths.sort();
        found.extend(paths);
    }
    found
}

/// A cheap stand-in for a schema directory's contents: file paths with their size and
/// modification time, so a cache can tell "nothing changed" from "reparse" without reading and
/// parsing every file on every query.
pub fn fingerprint(paths: &[PathBuf]) -> Vec<(PathBuf, u64, u64)> {
    paths
        .iter()
        .map(|path| {
            let meta = fs::metadata(path).ok();
            let len = meta.as_ref().map_or(0, fs::Metadata::len);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos() as u64);
            (path.clone(), len, modified)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, schema_paths};
    use sill_testkit::TempDir;
    use std::fs;

    #[test]
    fn only_schema_files_are_listed_and_a_missing_directory_is_skipped() {
        let dir = TempDir::new("schema-files-list");
        fs::write(dir.path().join("b.settings.toml"), "").unwrap();
        fs::write(dir.path().join("a.settings.toml"), "").unwrap();
        fs::write(dir.path().join("notes.toml"), "").unwrap();
        let missing = dir.path().join("absent");
        let listed = schema_paths(&[missing, dir.path().to_path_buf()]);
        assert_eq!(
            listed,
            vec![
                dir.path().join("a.settings.toml"),
                dir.path().join("b.settings.toml")
            ]
        );
    }

    #[test]
    fn the_fingerprint_changes_when_a_schema_file_does() {
        let dir = TempDir::new("schema-files-fingerprint");
        let file = dir.path().join("a.settings.toml");
        fs::write(&file, "one").unwrap();
        let before = fingerprint(std::slice::from_ref(&file));
        fs::write(&file, "one and more").unwrap();
        let after = fingerprint(std::slice::from_ref(&file));
        assert_ne!(before, after);
        assert_eq!(after[0].1, "one and more".len() as u64);
    }
}
