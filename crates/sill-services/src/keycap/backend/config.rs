//! The person's `keycap.toml`: read as text (keycap parses it, leniently) and watched for edits.

use std::io::ErrorKind;
use std::path::Path;

use crate::debounced::Debounced;
use crate::error::ServiceError;

/// The file's name inside keycap's config folder.
const FILE_NAME: &str = "keycap.toml";

/// The file's text; `None` when there is no such file or it cannot be read.
pub(crate) fn read(dir: &Path) -> Option<String> {
    let path = dir.join(FILE_NAME);
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            eprintln!("sill: keycap: {}: {error}", path.display());
            None
        }
    }
}

/// Watch the file in `dir`; the folder is made so the watch has something to stand on.
pub(crate) fn watch(dir: &Path) -> Result<Debounced, ServiceError> {
    Debounced::file(dir, FILE_NAME)
}
