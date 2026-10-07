//! `scan`/`parse_entry` against a fixture tree of `.desktop` files: two applications
//! directories (`home` takes precedence over `system`, matching `$XDG_DATA_HOME` over
//! `$XDG_DATA_DIRS`), visibility (`NoDisplay`, `Hidden`, `OnlyShowIn`/`NotShowIn` against
//! `COSMIC`), a non-`Application` type, a `TryExec` that is not installed, a nested directory
//! id, desktop actions and every other field `App` carries.
//!
//! No fixture sets `Icon=`, so every entry's icon resolves to `AppIcon::None` regardless of
//! this machine's installed themes: icon resolution itself is `tests/icons.rs`'s job.

use std::path::{Path, PathBuf};

use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
use sill_services::apps::{IconTheme, applications_dirs, scan};

fn fixture_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/applications");
    vec![root.join("home"), root.join("system")]
}

fn theme() -> IconTheme {
    IconTheme("hicolor".to_owned())
}

fn find<'a>(apps: &'a [App], id: &str) -> &'a App {
    apps.iter().find(|a| a.id.0 == id).unwrap_or_else(|| {
        panic!(
            "{id} not found among {:?}",
            apps.iter().map(|a| &a.id.0).collect::<Vec<_>>()
        )
    })
}

#[test]
fn scan_walks_every_dir_and_yields_them_in_precedence_order() {
    let found = scan(&fixture_dirs(), &theme(), &[]);

    let firefoxes: Vec<_> = found.iter().filter(|a| a.id.0 == "firefox").collect();
    assert_eq!(firefoxes.len(), 2, "both firefox.desktop files are yielded");
    assert_eq!(
        firefoxes[0].name, "Firefox (home)",
        "the home directory is scanned before the system one"
    );
    assert_eq!(firefoxes[1].name, "Firefox (system)");

    // Dedup itself is `apps::step`'s job (already frozen and tested): feeding this scan
    // through it keeps whichever came first, i.e. home's.
    let index = sill_model::apps::step(AppIndex::default(), AppsEvent::Scanned(found.clone()));
    let deduped: Vec<_> = index.apps.iter().filter(|a| a.id.0 == "firefox").collect();
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].name, "Firefox (home)");

    assert!(found.iter().any(|a| a.id.0 == "only-home"));
    assert!(found.iter().any(|a| a.id.0 == "only-system"));
}

#[test]
fn a_nested_directory_becomes_a_dashed_id() {
    let found = scan(&fixture_dirs(), &theme(), &[]);
    assert_eq!(find(&found, "kde-settings").name, "KDE Settings");
}

#[test]
fn visibility_rules_hide_but_do_not_exclude_except_a_non_application_type_and_a_dead_try_exec() {
    let found = scan(&fixture_dirs(), &theme(), &[]);

    let cases: &[(&str, Listing)] = &[
        ("hidden-app", Listing::Hidden),
        ("no-display-app", Listing::Hidden),
        ("only-gnome", Listing::Hidden),
        ("not-cosmic", Listing::Hidden),
        ("only-cosmic", Listing::Shown),
        ("only-home", Listing::Shown),
    ];
    for (id, want) in cases {
        assert_eq!(find(&found, id).listing, *want, "{id}");
    }

    assert!(
        !found.iter().any(|a| a.id.0 == "link"),
        "Type=Link is not an Application"
    );
    assert!(
        !found.iter().any(|a| a.id.0 == "missing-binary"),
        "a TryExec binary that is not on PATH excludes the entry entirely"
    );
}

#[test]
fn actions_without_a_name_are_dropped_and_the_rest_keep_their_declared_order() {
    let found = scan(&fixture_dirs(), &theme(), &[]);
    let app = find(&found, "actions-app");
    assert_eq!(app.actions.len(), 1, "the nameless action is dropped");
    assert_eq!(app.actions[0].id, "new-window");
    assert_eq!(app.actions[0].name, "New Window");
    assert_eq!(app.actions[0].exec, "actions-app --new-window");
}

#[test]
fn every_other_field_round_trips_through_parse_entry() {
    let found = scan(&fixture_dirs(), &theme(), &[]);
    let app = find(&found, "rich-app");

    assert_eq!(app.generic_name, "Text Editor");
    assert_eq!(app.comment, "Edits text richly");
    assert_eq!(app.exec, "rich-app %F");
    assert_eq!(app.icon, AppIcon::None);
    assert_eq!(app.startup_wm_class.as_deref(), Some("RichApp"));
    assert_eq!(app.flatpak.as_deref(), Some("org.example.RichApp"));
    assert_eq!(app.terminal, Terminal::Yes);
    assert_eq!(app.activation, DbusActivation::DBus);
    assert_eq!(app.mime_types, vec!["text/plain", "text/markdown"]);
    assert_eq!(app.keywords, vec!["edit", "write", "notes"]);
    assert_eq!(app.listing, Listing::Shown);
}

#[test]
fn applications_dirs_is_non_empty_and_does_not_panic() {
    // Not a fixture test: just proves the real function runs on this machine's real
    // environment and returns something (the desktop entry spec guarantees at least
    // `/usr/share/applications` is always a candidate, whether or not it exists).
    assert!(!applications_dirs(std::path::Path::new("/nowhere")).is_empty());
}
