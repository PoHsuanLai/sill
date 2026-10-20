//! The home Trash, as the freedesktop Trash specification lays it out: `$XDG_DATA_HOME/Trash`
//! with `files/` (what was trashed) and `info/` (one `.trashinfo` per item). Emptying removes
//! every entry of both, and the `directorysizes` cache. The per-volume `.Trash-$uid` folders of
//! other mounts are not touched.

use std::path::{Path, PathBuf};

use crate::env::Dirs;
use crate::error::ServiceError;

/// The Trash folder under `dirs`' data directory.
pub fn root(dirs: &Dirs) -> PathBuf {
    dirs.data.join("Trash")
}

/// Whether nothing is in the Trash. A Trash that does not exist is empty.
pub fn is_empty(dirs: &Dirs) -> bool {
    match std::fs::read_dir(root(dirs).join("files")) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    }
}

/// Remove everything in the Trash. Every entry is tried; the first failure is returned.
pub fn empty(dirs: &Dirs) -> Result<(), ServiceError> {
    let root = root(dirs);
    let failures = [
        clear(&root.join("files")),
        clear(&root.join("info")),
        remove_cache(&root.join("directorysizes")),
    ];
    failures
        .into_iter()
        .find_map(Result::err)
        .map_or(Ok(()), Err)
}

/// [`empty`] on a thread of its own; a failure is logged, there being nobody to tell.
pub fn empty_in_background(dirs: Dirs) {
    std::thread::spawn(move || {
        if let Err(error) = empty(&dirs) {
            eprintln!("sill: Empty Trash: {error}");
        }
    });
}

/// Remove every entry of `folder`, never following a link out of it.
fn clear(folder: &Path) -> Result<(), ServiceError> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io(folder, error)),
    };
    let results: Vec<Result<(), ServiceError>> = entries
        .map(|entry| {
            let entry = entry.map_err(|e| io(folder, e))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| io(&path, e))?;
            let gone = if kind.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            gone.map_err(|e| io(&path, e))
        })
        .collect();
    results
        .into_iter()
        .find_map(Result::err)
        .map_or(Ok(()), Err)
}

fn remove_cache(file: &Path) -> Result<(), ServiceError> {
    match std::fs::remove_file(file) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(io(file, error)),
        _ => Ok(()),
    }
}

fn io(path: &Path, source: std::io::Error) -> ServiceError {
    ServiceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
