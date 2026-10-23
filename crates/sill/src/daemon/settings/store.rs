//! The settings file the daemon reads and writes: one `ds_settings::Store` on the config root
//! `Env` already resolved, and the log lines a load leaves.
//!
//! What a file held that did not become a setting (a key nobody reads, a value a key refused, a
//! file that is not TOML) goes to the daemon's log on stderr, one line each, prefixed `sill:`. It
//! is reported on every read, the watch's re-reads included. Writes go through `patch`, which
//! sets only the keys it changes and keeps the rest of the file. There is no `sill` verb that
//! reads the file for its diagnostics.

use std::path::Path;

#[cfg(test)]
use ds_settings::SettingsError;
use ds_settings::{AppName, ConfigRoot, Loaded, SettingsDoc, Store};
use sill_settings::{APP, ShellFile};

/// The store of `sill/settings.toml` under `config`, `$XDG_CONFIG_HOME` as `Env` resolved it (a
/// scratch directory in a test or a dev script).
pub fn store(config: &Path) -> Store {
    Store::new(ConfigRoot::Scratch(config.to_path_buf()), APP)
}

/// The store of `quire/style.css` under `config`: the person's desktop-wide style, which every
/// quire app reads beside its own.
pub fn desktop_store(config: &Path) -> Store {
    Store::new(ConfigRoot::Scratch(config.to_path_buf()), AppName::QUIRE)
}

/// Print what `loaded` reports, one line per key.
pub fn report<D: SettingsDoc>(loaded: &Loaded<D>) {
    for line in loaded.diagnostics(D::FILE) {
        eprintln!("sill: {line}");
    }
}

/// Print what `loaded` reports, and the `keyboard.overrides` rows that did not become an override.
pub fn report_shell(loaded: &Loaded<ShellFile>) {
    report(loaded);
    for line in sill_settings::overrides(&loaded.value.keyboard).1 {
        eprintln!("sill: {}: {line}", ShellFile::FILE.0);
    }
}

/// The stored settings, or the defaults when there are none or they cannot be read; what the file
/// held that was not used is reported.
pub fn load(store: &Store) -> ShellFile {
    let loaded = store.load::<ShellFile>();
    report_shell(&loaded);
    loaded.value
}

/// Write all of `file`: a fixture for a test that wants a file to patch. The daemon never saves
/// the whole file; it patches (`patch`).
#[cfg(test)]
pub fn save(store: &Store, file: &ShellFile) -> Result<(), SettingsError> {
    let saved = store.save(file)?;
    if let Some(kept) = saved.kept_at {
        eprintln!(
            "sill: {}: could not be read, kept at {}",
            ShellFile::FILE.0,
            kept.display()
        );
    }
    Ok(())
}
