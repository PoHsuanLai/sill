//! The one way the daemon changes `sill/settings.toml`: set the key it means to change, in place,
//! through quire's `SettingsWriter`, and leave every other byte of the file as it was.
//!
//! The file has more than one writer (Settings writes its keys the same way; a person edits it by
//! hand), so loading the whole [`ShellFile`] and saving it back would put the old text of every
//! key a concurrent edit changed back. The writer reads the file under the lock every atomic
//! writer takes, sets only the keys it is given and renames the result into place.

use ds_settings::schema::KeyPath;
use ds_settings::{SettingsError, SettingsWriter, Store};
use serde::Serialize;
use sill_settings::ShellFile;

/// The key could not be written; the file was left as it was.
#[derive(Debug, thiserror::Error)]
pub enum WriteKeyError {
    /// The value has no form the file stores.
    #[error("{key} cannot be stored: {source}")]
    Value {
        key: String,
        source: toml::ser::Error,
    },
    /// The file could not be read, parsed or written.
    #[error("{0}")]
    File(#[from] SettingsError),
}

/// Set `key` (a dotted path such as `notifications.dnd`) to `value` in the store's
/// `settings.toml`, creating the file when there is none. A file that cannot be read or is not
/// TOML is left alone and the write fails.
pub fn write_key<T: Serialize>(store: &Store, key: &str, value: &T) -> Result<(), WriteKeyError> {
    let value = toml::Value::try_from(value).map_err(|source| WriteKeyError::Value {
        key: key.to_owned(),
        source,
    })?;
    SettingsWriter::of::<ShellFile>(store)?.patch(&KeyPath(key.to_owned()), &value)?;
    Ok(())
}
