//! A settings domain as a reader sees it: one table of `settings.toml`, found in a
//! [`ShellFile`] by type.

use crate::file::ShellFile;

/// One table of the settings file: `[bar]`, `[dock]`, .... A surface reads its domain through
/// `use_section::<S>()` and a policy through `Policy::of`, so neither names a field of
/// `ShellFile`. A new key is a field of a domain, never a new `Section`.
pub trait Section: Clone + PartialEq + Send + Sync + 'static {
    /// The table's name in the file: `bar`.
    const KEY: &'static str;

    /// This domain's value in `file`.
    fn of(file: &ShellFile) -> &Self;
}
