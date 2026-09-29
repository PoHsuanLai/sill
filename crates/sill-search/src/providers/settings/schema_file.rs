//! Reading `*.settings.toml` files directly, without depending on `ds-settings`.
//!
//! design/22-SETTINGS.md §9 makes the schema a plain data file: "a program does not register
//! its settings with a Settings app at runtime... it ships a schema file". `ds_settings::schema`
//! is the typed reader for that file, but `ds-settings` still depends on `ds`, which brings
//! `dioxus`, and `scripts/check-boundary.sh` forbids `sill-services` and `sill-search` from
//! reaching either (FINDINGS "The settings provider reads schema files itself"). So this module
//! re-reads the same on-disk TOML shape (`crates/ds-settings/src/schema/key.rs`, `program.rs`)
//! independently, taking only the fields the launcher's settings search needs (path, label,
//! help, page) and leaving every other field (`kind`, `default`, `deprecated`) untouched by not
//! naming it — the same forward-compatible reading every lenient settings load in this
//! workspace does.

use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::Deserialize;

/// One `[[key]]` entry, as much of it as the settings provider reads.
#[derive(Debug, Clone, Deserialize)]
pub struct RawKey {
    pub path: String,
    pub label: String,
    pub help: String,
    pub page: RawPage,
}

/// A key's `page` table: `{ kind = "dock" }`, or `{ kind = "app", v = "<id>" }` for
/// `ds_settings::schema::Page::App`.
#[derive(Debug, Clone, Deserialize)]
pub struct RawPage {
    pub kind: String,
    #[serde(default)]
    pub v: Option<String>,
}

/// One `<app-id>.settings.toml` file, as much of it as the settings provider reads.
#[derive(Debug, Clone, Deserialize)]
pub struct RawSchema {
    #[serde(default)]
    pub key: Vec<RawKey>,
}

/// The display name for a key's page: the fixed pages by their doc name, and the app id for a
/// third-party page (design/22 §9.3's page list).
pub fn page_label(page: &RawPage) -> String {
    match page.kind.as_str() {
        "appearance" => "Appearance".to_owned(),
        "dock" => "Dock".to_owned(),
        "mouse_and_gestures" => "Mouse & Gestures".to_owned(),
        "keyboard_and_shortcuts" => "Keyboard & Shortcuts".to_owned(),
        "notifications" => "Notifications".to_owned(),
        "spaces" => "Spaces".to_owned(),
        "accounts" => "Accounts".to_owned(),
        "apps" => "Apps".to_owned(),
        "app" => page.v.clone().unwrap_or_else(|| "App".to_owned()),
        other => other.to_owned(),
    }
}

/// Parses one schema file's text. A file that does not parse is the caller's to report and
/// skip (design/22 §9.2: "a schema without a program... is skipped with a warning"), not a
/// panic: it may belong to a version of a program this one predates.
pub fn parse_schema(text: &str) -> Result<RawSchema, toml::de::Error> {
    toml::from_str(text)
}

/// Every `*.settings.toml` file directly under each of `dirs`, in directory order and then file
/// name order within a directory — the same order `ds_settings::schema::discover` uses. A
/// directory that does not exist is skipped silently.
pub fn schema_paths(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with(".settings.toml"))
            })
            .collect();
        paths.sort();
        found.extend(paths);
    }
    found
}

/// `<data_home>/quire/settings` (the user's data directory) then every `$XDG_DATA_DIRS`-listed
/// `<dir>/quire/settings` (`ds_settings::schema::data_dirs`'s own directories: every program's
/// schema, quire's and sill's alike, installs under the shared `quire/settings` namespace,
/// design/22 §9.2). `$XDG_DATA_DIRS` is a system search path, read like `$PATH`.
pub fn data_dirs(data_home: &Path) -> Vec<PathBuf> {
    data_dirs_from(data_home, env::var_os("XDG_DATA_DIRS"))
}

fn data_dirs_from(data_home: &Path, xdg_data_dirs: Option<std::ffi::OsString>) -> Vec<PathBuf> {
    let dirs_list = xdg_data_dirs
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    std::iter::once(data_home.to_path_buf())
        .chain(
            dirs_list
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        )
        .map(|base| base.join("quire").join("settings"))
        .collect()
}

/// A cheap stand-in for a schema directory's contents: file paths with their size and
/// modification time, so a cache can tell "nothing changed" from "reparse" without reading and
/// parsing every file on every query.
pub fn fingerprint(paths: &[PathBuf]) -> Vec<(PathBuf, u64, u64)> {
    paths
        .iter()
        .map(|path| {
            let meta = fs::metadata(path).ok();
            let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            (path.clone(), len, modified)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{RawPage, data_dirs_from, page_label, parse_schema};

    #[test]
    fn xdg_data_home_comes_before_xdg_data_dirs() {
        let dirs = data_dirs_from(
            std::path::Path::new("/xdg-data-home"),
            Some(OsString::from("/a:/b")),
        );
        assert_eq!(
            dirs,
            vec![
                std::path::PathBuf::from("/xdg-data-home/quire/settings"),
                std::path::PathBuf::from("/a/quire/settings"),
                std::path::PathBuf::from("/b/quire/settings"),
            ]
        );
    }

    #[test]
    fn a_key_table_parses_its_path_label_help_and_page() {
        let text = r#"
app = "sill"
file = "sill/settings.toml"
version = 1

[[key]]
path = "dock.magnification"
default = "on"
label = "Magnification"
help = "Icons grow as the pointer nears them."
section = "Magnification"
exposure = "basic"

[key.kind]
kind = "toggle"

[key.kind.v]
variants = ["on", "off"]

[key.page]
kind = "dock"

[key.deprecated]
kind = "no"
"#;
        let schema = parse_schema(text).unwrap();
        assert_eq!(schema.key.len(), 1);
        let key = &schema.key[0];
        assert_eq!(key.path, "dock.magnification");
        assert_eq!(key.label, "Magnification");
        assert_eq!(key.help, "Icons grow as the pointer nears them.");
        assert_eq!(key.page.kind, "dock");
    }

    #[test]
    fn a_malformed_file_does_not_parse() {
        assert!(parse_schema("not = [valid").is_err());
    }

    #[test]
    fn page_labels_match_the_design_22_page_list() {
        const CASES: &[(&str, Option<&str>, &str)] = &[
            ("appearance", None, "Appearance"),
            ("dock", None, "Dock"),
            ("mouse_and_gestures", None, "Mouse & Gestures"),
            ("keyboard_and_shortcuts", None, "Keyboard & Shortcuts"),
            ("notifications", None, "Notifications"),
            ("spaces", None, "Spaces"),
            ("accounts", None, "Accounts"),
            ("apps", None, "Apps"),
            ("app", Some("com.example.App"), "com.example.App"),
        ];
        for &(kind, v, want) in CASES {
            let page = RawPage {
                kind: kind.to_owned(),
                v: v.map(str::to_owned),
            };
            assert_eq!(page_label(&page), want, "{kind}");
        }
    }
}
