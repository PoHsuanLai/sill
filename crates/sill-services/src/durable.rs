//! Writing a state file so that a crash, a full disk or a power cut leaves the old contents or the
//! new, never half of either: a uniquely named temporary file beside the target, flushed to disk,
//! renamed over it, and the folder flushed too. The file is private to its owner unless it already
//! had other permissions, and a symbolic link in the file's place is written through.
//!
//! A file that fails to read back is not overwritten by the next save without a trace:
//! [`set_aside`] moves it to a `.damaged` name first, so a person can still recover it.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::error::ServiceError;

/// The permissions of a file just written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Readable and writable by its owner alone.
    OwnerOnly,
    /// What the file had before (a file the person made or a program shares), else owner alone.
    AsFound,
}

/// The mode a file at `target` gets, given what is there now.
fn mode_for(target: &Path, access: Access) -> u32 {
    const OWNER_ONLY: u32 = 0o600;
    match access {
        Access::OwnerOnly => OWNER_ONLY,
        Access::AsFound => {
            fs::metadata(target).map_or(OWNER_ONLY, |found| found.permissions().mode() & 0o7777)
        }
    }
}

/// `path` with a link followed to the file it names, so a write replaces the file and keeps the
/// link; a path that does not exist (yet) is itself.
fn resolved(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A name for the temporary file beside `target` that no other writer in any process uses at the
/// same time: the process, the thread (a write does not yield, so a thread has one at a time),
/// and a leading dot so nothing lists it.
fn temp_beside(target: &Path) -> PathBuf {
    let name = target.file_name().map_or_else(
        || "state".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let unique = format!(
        ".{name}.{}-{:?}.part",
        std::process::id(),
        std::thread::current().id()
    );
    target.with_file_name(unique)
}

fn write_temp(temp: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(temp)?;
    // Set after the open, which the process's umask would have trimmed.
    file.set_permissions(Permissions::from_mode(mode))?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Replace the file at `path` (its folder created) with `bytes`, durably and atomically.
pub fn write(path: &Path, bytes: &[u8], access: Access) -> io::Result<()> {
    let target = resolved(path);
    let folder = match target.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
        Some(_) | None => PathBuf::from("."),
    };
    fs::create_dir_all(&folder)?;
    let mode = mode_for(&target, access);
    let temp = temp_beside(&target);
    let renamed = write_temp(&temp, bytes, mode).and_then(|()| fs::rename(&temp, &target));
    if renamed.is_err() {
        let _ = fs::remove_file(&temp);
    }
    renamed?;
    File::open(&folder)?.sync_all()
}

/// [`write`], naming the file in the error.
pub fn write_file(path: &Path, bytes: &[u8], access: Access) -> Result<(), ServiceError> {
    write(path, bytes, access).map_err(|source| ServiceError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Move the file at `path`, which could not be read back, to `<name>.damaged` beside it, so the
/// next save does not destroy it. A file that is not there is nothing to set aside.
pub fn set_aside(path: &Path) {
    let Some(name) = path.file_name() else {
        return;
    };
    let mut aside = name.to_os_string();
    aside.push(".damaged");
    if path.exists() && fs::rename(path, path.with_file_name(aside)).is_ok() {
        eprintln!(
            "sill: {} could not be read; kept as .damaged",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use sill_testkit::TempDir;

    use super::{Access, set_aside, write};

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or_default()
    }

    fn names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[test]
    fn a_new_file_is_private_whole_and_leaves_nothing_beside_it() {
        let dir = TempDir::new("durable-new");
        let path = dir.path().join("state/sill/a.json");
        write(&path, b"one", Access::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        write(&path, b"two", Access::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), b"two");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(names(&dir.path().join("state/sill")), vec!["a.json"]);
    }

    #[test]
    fn a_file_keeps_the_permissions_it_had_when_asked_to() {
        let dir = TempDir::new("durable-as-found");
        let path = dir.path().join("shared.toml");
        std::fs::write(&path, b"x").unwrap_or_else(|e| panic!("{e}"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .unwrap_or_else(|e| panic!("{e}"));
        write(&path, b"y", Access::AsFound).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(mode(&path), 0o644);
        write(&path, b"z", Access::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn a_link_in_the_files_place_is_written_through() {
        let dir = TempDir::new("durable-link");
        let real = dir.path().join("dotfiles/settings.toml");
        std::fs::create_dir_all(dir.path().join("dotfiles")).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&real, b"old").unwrap_or_else(|e| panic!("{e}"));
        let link = dir.path().join("settings.toml");
        symlink(&real, &link).unwrap_or_else(|e| panic!("{e}"));
        write(&link, b"new", Access::AsFound).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            std::fs::symlink_metadata(&link)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "the link is still a link"
        );
        assert_eq!(std::fs::read(&real).unwrap_or_default(), b"new");
    }

    #[test]
    fn a_file_that_could_not_be_read_is_kept_aside() {
        let dir = TempDir::new("durable-aside");
        let path = dir.path().join("a.json");
        std::fs::write(&path, b"{ broken").unwrap_or_else(|e| panic!("{e}"));
        set_aside(&path);
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(dir.path().join("a.json.damaged")).unwrap_or_default(),
            b"{ broken"
        );
        set_aside(&path);
    }
}
