//! The index's data and its one reducer.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

/// Every installed application, sorted by id, one entry per id (the first found in XDG
/// precedence order wins).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppIndex {
    pub apps: Vec<App>,
}

impl AppIndex {
    /// The entry with `id`.
    pub fn get(&self, id: &AppId) -> Option<&App> {
        self.apps
            .binary_search_by(|app| app.id.cmp(id))
            .ok()
            .and_then(|i| self.apps.get(i))
    }
}

/// One desktop entry, as the dock, the launcher and `launch` need it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    pub id: AppId,
    /// `Name`, localised.
    pub name: String,
    /// `GenericName`, localised; empty for none.
    pub generic_name: String,
    /// `Comment`, localised; empty for none.
    pub comment: String,
    /// `Exec`, after desktop-file escapes, before field codes (see [`crate::apps::expand_exec`]).
    pub exec: String,
    pub icon: AppIcon,
    /// `StartupWMClass`, for `match_app`; `None` when the entry does not say.
    pub startup_wm_class: Option<String>,
    /// `X-Flatpak`, the Flatpak app id, for `match_app`; `None` for a host app.
    pub flatpak: Option<String>,
    pub terminal: Terminal,
    pub activation: DbusActivation,
    /// `Actions=` in declared order, each with its `[Desktop Action]` group.
    pub actions: Vec<DesktopActionEntry>,
    /// `MimeType=`, for file drops onto a dock tile.
    pub mime_types: Vec<String>,
    /// `Keywords=`, localised, matched by the launcher.
    pub keywords: Vec<String>,
    /// `NoDisplay`/`Hidden`/`OnlyShowIn`: hidden entries still match windows, never list.
    pub listing: Listing,
    /// The `.desktop` file (for `%k`, Show in Files, autostart copies).
    pub path: PathBuf,
}

/// An entry's icon, resolved against the icon theme at scan time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AppIcon {
    /// No `Icon=`, or nothing found in the theme: the dock draws the symbolic fallback.
    None,
    /// A file on disk: an absolute `Icon=`, or the theme lookup's answer.
    File(PathBuf),
}

/// `Terminal=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terminal {
    #[default]
    No,
    /// Run inside the terminal wrapper.
    Yes,
}

/// `DBusActivatable=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbusActivation {
    /// Start with `Exec`.
    #[default]
    Exec,
    /// Start with `org.freedesktop.Application.Activate` on the bus name `id`.
    DBus,
}

/// Whether an entry is listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Listing {
    #[default]
    Shown,
    Hidden,
}

/// One `[Desktop Action <id>]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopActionEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
}

/// One change to the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AppsEvent {
    /// A whole rescan, in XDG precedence order (earlier wins a duplicate id).
    Scanned(Vec<App>),
    /// One entry added or changed (boxed: an `App` is large beside the other variants).
    Upserted(Box<App>),
    Removed(AppId),
}

/// `index` after `event`; the result is always sorted by id with no duplicate ids.
pub fn step(index: AppIndex, event: AppsEvent) -> AppIndex {
    let AppIndex { mut apps } = index;
    match event {
        AppsEvent::Scanned(found) => {
            apps = Vec::with_capacity(found.len());
            for app in found {
                if let Err(at) = apps.binary_search_by(|a: &App| a.id.cmp(&app.id)) {
                    apps.insert(at, app);
                }
            }
        }
        AppsEvent::Upserted(app) => match apps.binary_search_by(|a| a.id.cmp(&app.id)) {
            Ok(at) => apps[at] = *app,
            Err(at) => apps.insert(at, *app),
        },
        AppsEvent::Removed(id) => apps.retain(|a| a.id != id),
    }
    AppIndex { apps }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal, step};
    use sill_launcher::AppId;
    use std::path::PathBuf;

    /// A minimal entry with `id` and `name`, for this module's and `match_app`'s tests.
    pub(crate) fn app(id: &str, name: &str) -> App {
        App {
            id: AppId(id.to_owned()),
            name: name.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: id.to_owned(),
            icon: AppIcon::None,
            startup_wm_class: None,
            flatpak: None,
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        }
    }

    fn ids(index: &AppIndex) -> Vec<(&str, &str)> {
        index
            .apps
            .iter()
            .map(|a| (a.id.0.as_str(), a.name.as_str()))
            .collect()
    }

    #[test]
    fn a_scan_is_sorted_and_the_first_duplicate_wins() {
        let index = step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![
                app("foot", "Foot (user)"),
                app("org.mozilla.firefox", "Firefox"),
                app("foot", "Foot (system)"),
                app("cosmic-term", "Terminal"),
            ]),
        );
        assert_eq!(
            ids(&index),
            vec![
                ("cosmic-term", "Terminal"),
                ("foot", "Foot (user)"),
                ("org.mozilla.firefox", "Firefox"),
            ]
        );
    }

    #[test]
    fn upserts_and_removals_keep_the_order() {
        let base = step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![app("b", "B"), app("d", "D")]),
        );
        type Case<'a> = (&'a str, AppsEvent, Vec<(&'a str, &'a str)>);
        let cases: Vec<Case> = vec![
            (
                "insert in the middle",
                AppsEvent::Upserted(Box::new(app("c", "C"))),
                vec![("b", "B"), ("c", "C"), ("d", "D")],
            ),
            (
                "replace in place",
                AppsEvent::Upserted(Box::new(app("d", "D2"))),
                vec![("b", "B"), ("d", "D2")],
            ),
            (
                "remove",
                AppsEvent::Removed(AppId("b".to_owned())),
                vec![("d", "D")],
            ),
            (
                "remove unknown",
                AppsEvent::Removed(AppId("zz".to_owned())),
                vec![("b", "B"), ("d", "D")],
            ),
            (
                "a rescan replaces everything",
                AppsEvent::Scanned(vec![app("a", "A")]),
                vec![("a", "A")],
            ),
        ];
        for (name, event, want) in cases {
            assert_eq!(ids(&step(base.clone(), event)), want, "{name}");
        }
    }

    #[test]
    fn get_finds_by_id() {
        let index = step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![app("b", "B"), app("d", "D")]),
        );
        assert_eq!(
            index.get(&AppId("d".to_owned())).map(|a| a.name.as_str()),
            Some("D")
        );
        assert!(index.get(&AppId("c".to_owned())).is_none());
    }
}
