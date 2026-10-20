//! Reading desktop entries from disk (effects: the filesystem).

use std::path::PathBuf;

use ds_helpers::{EntryLocale, load_entries};
use sill_model::apps::App;

use super::entry::parse_entry;
use super::icon_roots::IconRoots;
use super::icons::IconTheme;

/// Every entry under `dirs` (the applications folders, [`Dirs::applications`](crate::env::Dirs)),
/// in precedence order with each id once, icons resolved in `theme` (an absolute `Icon=` only
/// inside `roots`), with the desktop's current locales. A folder that does not exist, or a
/// `.desktop` file that cannot be read (or is over [`ds_helpers::DESKTOP_ENTRY_CAP`], or is not text), is
/// skipped rather than failing the scan. The walk, the ids and the shadowing are
/// [`ds_helpers::load_entries`]'s, the same for every part of the desktop.
pub fn scan(
    dirs: &[PathBuf],
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[EntryLocale],
) -> Vec<App> {
    load_entries(dirs)
        .iter()
        .filter_map(|entry| parse_entry(entry, theme, roots, locales))
        .collect()
}
