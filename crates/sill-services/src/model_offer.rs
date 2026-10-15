//! The first start's offer of the assistant's model picker. The assistant has no default model,
//! so the first time a person enters the desktop Settings opens the first-run picker, once:
//! `$XDG_STATE_HOME/sill/model-offered.json` records that it was offered, whether the person
//! then picks or skips.
//!
//! sill does not ask whether a model is set (only Settings reads and changes that setting), so
//! the offer is made on the first start whatever the setting holds.

use std::path::{Path, PathBuf};

use sill_model::launch::LaunchCommand;
use sill_model::seed::{ModelOffered, first_model_offer};

use crate::control::Control;
use crate::env::Dirs;
use crate::error::ServiceError;
use crate::first_run::store_of;

/// The record of the offer, in sill's folder of `dirs`' state directory.
pub fn path_in(dirs: &Dirs) -> PathBuf {
    dirs.sill_state().join("model-offered.json")
}

/// On a start with no record of the offer, record it and have `launch` open the first-run
/// picker; on any later start, do nothing. The record comes first, so an offer that could not
/// be recorded is not made at all rather than made at every start.
pub fn offer_first_run(dirs: &Dirs, launch: &Control<LaunchCommand>) -> Result<(), ServiceError> {
    let path = path_in(dirs);
    let Some(page) = first_model_offer(store_of(&path)) else {
        return Ok(());
    };
    save(&path, &ModelOffered { page })?;
    launch.send(LaunchCommand::OpenSettingsPage(page))
}

/// Write `offered` to `path` through a temporary file and a rename.
fn save(path: &Path, offered: &ModelOffered) -> Result<(), ServiceError> {
    let io = |at: &Path| {
        let path = at.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
    }
    let text = serde_json::to_string_pretty(offered).map_err(|error| ServiceError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(error),
    })?;
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, text).map_err(io(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io(path))
}
