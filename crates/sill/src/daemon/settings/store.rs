//! The settings file the daemon reads and writes: one `ds_settings::Store` on the config root
//! `Env` already resolved, and the log lines a load leaves.
//!
//! What a file held that did not become a setting (a key nobody reads, a value a key refused, a
//! file that is not TOML) goes to the daemon's log on stderr, one line each, prefixed `sill:`. It
//! is reported on every read, the watch's re-reads included, and the next save drops the unknown
//! keys. There is no `sill` verb that reads the file for its diagnostics.

use std::path::Path;

use ds_settings::{ConfigRoot, Loaded, SettingsDoc, SettingsError, Store};
use sill_settings::{APP, ShellFile};

/// The store of `sill/settings.toml` under `config`, `$XDG_CONFIG_HOME` as `Env` resolved it (a
/// scratch directory in a test or a dev script).
pub fn store(config: &Path) -> Store {
    Store::new(ConfigRoot::Scratch(config.to_path_buf()), APP)
}

/// Print what `loaded` reports, one line per key.
pub fn report<D: SettingsDoc>(loaded: &Loaded<D>) {
    for line in loaded.diagnostics(D::FILE) {
        eprintln!("sill: {line}");
    }
}

/// The stored settings, or the defaults when there are none or they cannot be read; what the file
/// held that was not used is reported.
pub fn load(store: &Store) -> ShellFile {
    let loaded = store.load::<ShellFile>();
    report(&loaded);
    loaded.value
}

/// Write `file`, creating its directory if needed.
pub fn save(store: &Store, file: &ShellFile) -> Result<(), SettingsError> {
    store.save(file)
}
