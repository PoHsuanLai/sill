//! The one way the daemon changes `sill/settings.toml`: set the keys it means to change, in
//! place, and leave every other byte of the file as it was.
//!
//! The file has more than one writer (Settings writes its keys the same way; a person edits it by
//! hand). Loading the whole [`ShellFile`](sill_settings::ShellFile) and saving it back would put
//! the old text of every key a concurrent edit changed back, drop its comments and re-write a
//! refused value as its default. A patch reads the file under the lock every `update_atomic`
//! writer takes, sets only its own keys and renames the result into place.
//!
//! This is a small stand-in for the shared `SettingsWriter::patch` that quire will own.

use std::path::Path;

use ds_helpers::{NewFileMode, ReadCap, update_atomic};
use ds_settings::Store;
use serde::Serialize;
use sill_settings::ShellFile;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, TableLike, Value};

/// The most of `settings.toml` a patch reads.
const SETTINGS_CAP: ReadCap = ReadCap::mib(1);

/// The file could not be patched; it was left as it was.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct PatchError(String);

/// One key to set: a dotted path (`notifications.dnd`) and the value it takes.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyEdit {
    key: &'static str,
    value: toml::Value,
}

impl KeyEdit {
    /// `key` set to `value`, in the form the file stores it.
    pub fn of<T: Serialize>(key: &'static str, value: &T) -> Result<KeyEdit, PatchError> {
        let value = toml::Value::try_from(value)
            .map_err(|error| PatchError(format!("{key} cannot be stored: {error}")))?;
        Ok(KeyEdit { key, value })
    }
}

/// Set each of `edits` in the store's `settings.toml`, creating the file when there is none. A
/// file that cannot be read or is not TOML is left alone and the patch fails.
pub fn patch(store: &Store, edits: &[KeyEdit]) -> Result<(), PatchError> {
    let path = store
        .path::<ShellFile>()
        .ok_or_else(|| PatchError("there is no config directory".to_owned()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| PatchError(format!("{}: {error}", dir.display())))?;
    }
    update_atomic(&path, SETTINGS_CAP, NewFileMode::Umask, |text| {
        edited(&path, &text.unwrap_or_default(), edits)
    })
    .map_err(|error| PatchError(format!("{}: {error}", path.display())))
}

/// `text` with every key of `edits` set; every other byte is the file's own.
fn edited(path: &Path, text: &str, edits: &[KeyEdit]) -> Result<String, PatchError> {
    let mut doc: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
        PatchError(format!("{}: {error}", path.display()))
    })?;
    for edit in edits {
        let parts: Vec<&str> = edit.key.split('.').collect();
        let value =
            to_edit(&edit.value).map_err(|reason| PatchError(format!("{}: {reason}", edit.key)))?;
        set(doc.as_table_mut(), &parts, value).map_err(|()| {
            PatchError(format!(
                "{}: {} sits inside a value that is not a table",
                path.display(),
                edit.key
            ))
        })?;
    }
    Ok(doc.to_string())
}

/// Set `path` in `table`, making the tables on the way. An existing key keeps its place, spacing
/// and trailing comment; only its value is replaced.
fn set(table: &mut dyn TableLike, path: &[&str], value: Value) -> Result<(), ()> {
    match path {
        [] => Ok(()),
        [last] => {
            match table.get_mut(last) {
                Some(item) => {
                    let mut value = value;
                    if let Some(old) = item.as_value() {
                        *value.decor_mut() = old.decor().clone();
                    }
                    *item = Item::Value(value);
                }
                None => {
                    table.insert(last, Item::Value(value));
                }
            }
            Ok(())
        }
        [first, rest @ ..] => {
            if table.get(first).is_none() {
                let mut made = Table::new();
                made.set_implicit(true);
                table.insert(first, Item::Table(made));
            }
            let inner = table
                .get_mut(first)
                .and_then(Item::as_table_like_mut)
                .ok_or(())?;
            set(inner, rest, value)
        }
    }
}

/// `value` as the editable document stores it; a table becomes an inline table.
fn to_edit(value: &toml::Value) -> Result<Value, String> {
    Ok(match value {
        toml::Value::String(text) => Value::from(text.as_str()),
        toml::Value::Integer(number) => Value::from(*number),
        toml::Value::Float(number) => Value::from(*number),
        toml::Value::Boolean(flag) => Value::from(*flag),
        toml::Value::Datetime(stamp) => Value::from(
            stamp
                .to_string()
                .parse::<toml_edit::Datetime>()
                .map_err(|error| error.to_string())?,
        ),
        toml::Value::Array(items) => Value::Array(
            items
                .iter()
                .map(to_edit)
                .collect::<Result<Array, String>>()?,
        ),
        toml::Value::Table(table) => {
            let mut inline = InlineTable::new();
            for (name, inner) in table {
                inline.insert(name.as_str(), to_edit(inner)?);
            }
            Value::InlineTable(inline)
        }
    })
}
