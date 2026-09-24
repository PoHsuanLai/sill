//! Reading desktop entries from disk (effects: the filesystem).

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::path::{Path, PathBuf};

use crate::apps::icons::IconTheme;
use crate::apps::index::App;

/// `$XDG_DATA_HOME/applications` then each `$XDG_DATA_DIRS/applications`, in precedence order
/// (Flatpak's exports included when they are on `XDG_DATA_DIRS`).
pub fn applications_dirs() -> Vec<PathBuf> {
    todo!("M2 apps-index")
}

/// Every entry under `dirs`, in precedence order, icons resolved in `theme`, with the
/// desktop's current locales.
pub fn scan(dirs: &[PathBuf], theme: &IconTheme, locales: &[String]) -> Vec<App> {
    todo!("M2 apps-index: freedesktop_desktop_entry::Iter over dirs, parse_entry each")
}

/// One `.desktop` file's text as an [`App`] with id derived from `path` relative to its
/// applications directory (`kde/foo.desktop` is `kde-foo`); `None` for a non-Application type
/// or an entry without `Name`/`Exec`.
pub fn parse_entry(
    path: &Path,
    id: &str,
    text: &str,
    theme: &IconTheme,
    locales: &[String],
) -> Option<App> {
    todo!("M2 apps-index")
}
