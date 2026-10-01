//! `keyboard.*`: how sill treats the keyboard per app (design/22 §3.25). One key, a list of
//! tables: the keycap profile each app's keys are drawn with. The file keeps the person's text;
//! [`overrides`] turns it into the typed values the keycap service reads, once per load.

use crate::file::ShellFile;
use crate::section::Section;
use ds_settings::InvalidKey;
use ds_settings::schema::Page;
use keycap_protocol::ProfileName;
use serde::{Deserialize, Serialize};
use sill_launcher::AppId;
use sill_model::keyboard::UserOverrides;
use std::collections::BTreeMap;

/// One row of `keyboard.overrides`: an app and the keycap profile its windows get.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsRow)]
pub struct KeyboardOverride {
    /// The desktop-entry id (`org.gnome.Nautilus`).
    #[settings(label = "App")]
    pub app: String,
    /// A keycap profile name; one keycap does not define resolves to `default` there.
    #[settings(label = "Profile", pattern = "[a-z0-9_-]{1,64}")]
    pub profile: String,
}

/// A row of the file that is not a table of two texts, `app` and `profile`: it cost only itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedRow {
    /// Its place in the file's list, from 1.
    pub position: usize,
    /// What is wrong with it.
    pub reason: String,
}

/// The keyboard's per-app profiles.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(from = "KeyboardWire")]
#[settings(file = "sill/settings.toml", domain = "keyboard", page = Page::KeyboardAndShortcuts)]
pub struct KeyboardSettings {
    #[settings(
        label = "Per-app profiles",
        help = "One row per app: the desktop-entry id and the keycap profile its windows use. Beats the profile the app declares and its category; a name keycap does not define uses the default profile.",
        section = "Profiles",
        advanced
    )]
    pub overrides: Vec<KeyboardOverride>,
    /// The rows of the file that were not rows at all; never written back.
    #[settings(skip)]
    #[serde(skip_serializing)]
    pub rejected: Vec<RejectedRow>,
}

impl Section for KeyboardSettings {
    const KEY: &'static str = "keyboard";

    fn of(file: &ShellFile) -> &Self {
        &file.keyboard
    }
}

/// The table as the file spells it, a row at a time, so one malformed row costs that row and
/// not the key (the lenient reader takes a list as one value).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct KeyboardWire {
    overrides: Vec<WireRow>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WireRow {
    Row(KeyboardOverride),
    Other(toml::Value),
}

impl From<KeyboardWire> for KeyboardSettings {
    fn from(wire: KeyboardWire) -> Self {
        let mut settings = KeyboardSettings::default();
        for (index, row) in wire.overrides.into_iter().enumerate() {
            match row {
                WireRow::Row(row) => settings.overrides.push(row),
                WireRow::Other(value) => settings.rejected.push(RejectedRow {
                    position: index + 1,
                    reason: not_a_row(&value),
                }),
            }
        }
        settings
    }
}

/// Why `value` is not a row.
fn not_a_row(value: &toml::Value) -> String {
    let toml::Value::Table(table) = value else {
        return "a row is a table with `app` and `profile`".to_owned();
    };
    ["app", "profile"]
        .iter()
        .find_map(|column| match table.get(*column) {
            None => Some(format!("`{column}` is missing")),
            Some(toml::Value::String(_)) => None,
            Some(_) => Some(format!("`{column}` must be text")),
        })
        .unwrap_or_else(|| "a row is a table with `app` and `profile`".to_owned())
}

/// What a dropped row is told as: it costs that row, never the key.
fn dropped(reason: String) -> InvalidKey {
    InvalidKey {
        path: "keyboard.overrides".to_owned(),
        reason: format!("a row is dropped and the others kept: {reason}"),
    }
}

/// The typed overrides `settings` holds, and a line for every row that did not become one: a
/// row whose `profile` is not a profile name or whose `app` is empty is dropped, and when two
/// rows name one app the last wins. The keycap service never sees a string.
pub fn overrides(settings: &KeyboardSettings) -> (UserOverrides, Vec<InvalidKey>) {
    let mut found = BTreeMap::new();
    let mut lines: Vec<InvalidKey> = settings
        .rejected
        .iter()
        .map(|row| dropped(format!("row {}: {}", row.position, row.reason)))
        .collect();
    for row in &settings.overrides {
        if row.app.is_empty() {
            lines.push(dropped(format!(
                "`app` is empty (its profile is `{}`)",
                row.profile
            )));
            continue;
        }
        match ProfileName::parse(&row.profile) {
            Err(error) => lines.push(dropped(format!(
                "`{}`'s profile `{}` is not valid: {error}",
                row.app, row.profile
            ))),
            Ok(profile) => {
                if found.insert(AppId(row.app.clone()), profile).is_some() {
                    lines.push(InvalidKey {
                        path: "keyboard.overrides".to_owned(),
                        reason: format!(
                            "`{}` has more than one row, the last one is used",
                            row.app
                        ),
                    });
                }
            }
        }
    }
    (UserOverrides(found), lines)
}
