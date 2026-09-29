//! The apps provider's query table against the real fixture tree `tests/scan.rs` already
//! exercises (`sill-services/tests/fixtures/applications`): name, generic name and keyword matches, a
//! matching desktop action surfacing as its own row, and the visibility rules (`Hidden`,
//! `NoDisplay`, an unresolvable `TryExec`) keeping entries out of the launcher the same way
//! they keep them out of the dock.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sill_launcher::{Cancel, Frecency, Provider, Query, VecSink};
use sill_model::apps::{AppIndex, AppsEvent};
use sill_search::providers::AppsProvider;
use sill_services::apps::{IconTheme, scan};

fn fixture_dirs() -> Vec<PathBuf> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sill-services/tests/fixtures/applications");
    vec![root.join("home"), root.join("system")]
}

fn provider() -> AppsProvider {
    let found = scan(&fixture_dirs(), &IconTheme("hicolor".to_owned()), &[]);
    let index = sill_model::apps::step(AppIndex::default(), AppsEvent::Scanned(found));
    let (_tx, rx) = tokio::sync::watch::channel(index);
    AppsProvider {
        apps: rx,
        frecency: Arc::new(Mutex::new(Frecency::default())),
    }
}

fn titles(provider: &AppsProvider, text: &str) -> Vec<String> {
    let cancel = Cancel::new();
    let query = Query::begin(text, &cancel);
    let mut sink = VecSink::default();
    provider.query(&query, &mut sink);
    let mut titles: Vec<String> = sink
        .0
        .into_iter()
        .flat_map(|batch| batch.items)
        .map(|item| item.title)
        .collect();
    titles.sort();
    titles
}

#[test]
fn a_query_table_matches_the_fixture_tree() {
    let provider = provider();
    const CASES: &[(&str, &[&str])] = &[
        ("firefox", &["Firefox (home)"]),
        ("rich", &["Rich App"]),
        ("notes", &["Rich App"]),  // a keyword match
        ("editor", &["Rich App"]), // a generic-name match
        ("actions", &["Actions App"]),
        ("new window", &["New Window"]), // the desktop action, as its own row
        ("only-home", &[]), // the id (a hyphen, no space) is not matched, only names/keywords
        ("no such application anywhere", &[]),
    ];
    for &(text, want) in CASES {
        assert_eq!(titles(&provider, text), want, "{text:?}");
    }
}

#[test]
fn hidden_no_display_and_an_unresolvable_try_exec_never_list() {
    let provider = provider();
    for text in ["hidden", "no display", "missing binary"] {
        assert!(titles(&provider, text).is_empty(), "{text:?}");
    }
}

#[test]
fn firefox_home_shadows_firefox_system_the_same_way_the_dock_sees_it() {
    // `tests/scan.rs::scan_walks_every_dir_and_yields_them_in_precedence_order` proves the raw
    // scan keeps both; this proves the provider only ever offers the one `apps::step` kept.
    let provider = provider();
    assert_eq!(titles(&provider, "firefox"), vec!["Firefox (home)"]);
}
