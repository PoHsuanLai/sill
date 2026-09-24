//! Reading and writing `frecency.json` (effects: the filesystem).

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::path::Path;

use sill_launcher::Frecency;

use crate::error::ServiceError;

/// The file's name inside `$XDG_STATE_HOME/sill`.
pub const FILE_NAME: &str = "frecency.json";

/// The history in `dir`; missing or damaged is empty.
pub fn load_frecency(dir: &Path) -> Frecency {
    todo!("M4 launcher-core: read FILE_NAME, Frecency::from_json")
}

/// Write `frecency` to `dir` by temp file and rename.
pub fn save_frecency(dir: &Path, frecency: &Frecency) -> Result<(), ServiceError> {
    todo!("M4 launcher-core")
}
