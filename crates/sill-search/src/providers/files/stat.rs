//! A path an index printed, as a [`FileHit`]: its name, its type (the index's, else guessed
//! from the name, `inode/directory` for a folder) and its modification time, read with one
//! `stat` of the path itself (never of its contents).

use std::path::{Path, PathBuf};

use sill_launcher::{Mime, UnixTime};

use super::index::FileHit;
use crate::clock::unix_seconds;

/// The type a file with no known type has.
const UNKNOWN: &str = "application/octet-stream";

/// Whether a path is a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    Folder,
    File,
}

/// `path` as a hit, or `None` when it is gone (the index is older than the disk).
pub(super) fn hit_at(path: PathBuf) -> Option<FileHit> {
    let meta = std::fs::metadata(&path).ok()?;
    let shape = if meta.is_dir() {
        Shape::Folder
    } else {
        Shape::File
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(unix_seconds)
        .unwrap_or(UnixTime(0));
    let mime = mime_of(&path, shape);
    hit_of(path, mime, modified)
}

/// A hit for `path` with `mime` and `modified`, or `None` for a path with no file name (`/`).
pub(super) fn hit_of(path: PathBuf, mime: Mime, modified: UnixTime) -> Option<FileHit> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    Some(FileHit {
        path,
        name,
        mime,
        modified,
    })
}

/// A path's type from its name alone.
pub(super) fn mime_of(path: &Path, shape: Shape) -> Mime {
    match shape {
        Shape::Folder => Mime(Mime::FOLDER.to_owned()),
        Shape::File => Mime(
            mime_guess::from_path(path)
                .first_raw()
                .unwrap_or(UNKNOWN)
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sill_launcher::Mime;

    use super::{Shape, mime_of};

    #[test]
    fn a_type_comes_from_the_name_and_a_folder_is_a_directory() {
        let cases: &[(&str, Shape, &str)] = &[
            ("/h/a.pdf", Shape::File, "application/pdf"),
            ("/h/b.png", Shape::File, "image/png"),
            ("/h/notes.md", Shape::File, "text/markdown"),
            ("/h/no-extension", Shape::File, "application/octet-stream"),
            ("/h/Documents", Shape::Folder, "inode/directory"),
            ("/h/folder.pdf", Shape::Folder, "inode/directory"),
        ];
        for &(path, shape, want) in cases {
            assert_eq!(
                mime_of(Path::new(path), shape),
                Mime(want.to_owned()),
                "{path}"
            );
        }
    }
}
