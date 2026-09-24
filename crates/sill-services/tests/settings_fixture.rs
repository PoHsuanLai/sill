//! The settings provider against a fixture schema directory: two files
//! (`tests/fixtures/settings/{quire,sill}.settings.toml`) standing in for an installed quire
//! schema and an installed sill schema, discovered and indexed together — design/22-SETTINGS.md
//! §9.3's "the launcher's Settings provider reads the same schemas... no hand-kept table".
//!
//! The four deep-link examples design/22-SETTINGS.md itself names: §9.6's acceptance list
//! ("dark mode", "magnification", "natural scrolling") each resolve to one key; §9.3's own
//! "bluetooth" example is explicitly the *other* case — it names the one query meant to resolve
//! through a live D-Bus settings module (§9.4), which nothing in this workspace implements yet,
//! so it must resolve to no key here (FINDINGS.md "M4 providers-b").

use std::path::PathBuf;

use sill_launcher::{Activation, Cancel, Choice, Provider, Query, SettingsTarget, VecSink};
use sill_services::providers::SettingsProvider;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/settings")
}

fn query_target(provider: &SettingsProvider, text: &str) -> Vec<SettingsTarget> {
    let cancel = Cancel::new();
    let query = Query::begin(text, &cancel);
    let mut sink = VecSink::default();
    provider.query(&query, &mut sink);
    sink.0
        .into_iter()
        .flat_map(|batch| batch.items)
        .filter_map(|item| match provider.activate(&item, &Choice::Primary) {
            Activation::OpenSettings { target } => Some(target),
            _ => None,
        })
        .collect()
}

#[test]
fn dark_mode_resolves_to_appearance_theme() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    let targets = query_target(&provider, "dark mode");
    assert_eq!(
        targets.first(),
        Some(&SettingsTarget::Key("appearance.theme".to_owned())),
        "{targets:?}"
    );
}

#[test]
fn magnification_resolves_to_dock_magnification() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    let targets = query_target(&provider, "magnification");
    assert_eq!(
        targets.first(),
        Some(&SettingsTarget::Key("dock.magnification".to_owned())),
        "{targets:?}"
    );
}

#[test]
fn natural_scrolling_resolves_to_scroll_natural() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    let targets = query_target(&provider, "natural scrolling");
    assert_eq!(
        targets.first(),
        Some(&SettingsTarget::Key("scroll.natural".to_owned())),
        "{targets:?}"
    );
}

/// design/22-SETTINGS.md §9.3: "bluetooth" is the doc's own example of a query meant for a
/// *live* settings module (§9.4), not a file schema key — nothing in this fixture, or in the
/// real installed schemas today, has a static key mentioning it, so the deep link must not
/// invent one. This is a gap, not a silent omission: FINDINGS.md "M4 providers-b" reports it.
#[test]
fn bluetooth_does_not_deep_link_to_any_file_backed_key() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    let targets = query_target(&provider, "bluetooth");
    assert!(
        !targets.iter().any(|t| matches!(t, SettingsTarget::Key(_))),
        "{targets:?}"
    );
}

#[test]
fn both_fixture_schemas_are_discovered() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    // Every key from both files should be reachable by an exact path search.
    for path in [
        "appearance.theme",
        "appearance.accent",
        "dock.magnification",
        "scroll.natural",
        "control_center.modules",
    ] {
        let targets = query_target(&provider, path);
        assert!(
            targets.contains(&SettingsTarget::Key(path.to_owned())),
            "expected {path} to be found by its own path, got {targets:?}"
        );
    }
}

#[test]
fn a_third_party_page_key_carries_its_app_id_as_the_page_label() {
    let provider = SettingsProvider::with_dirs(vec![fixture_dir()]);
    let cancel = Cancel::new();
    let query = Query::begin("modules", &cancel);
    let mut sink = VecSink::default();
    provider.query(&query, &mut sink);
    let item = sink
        .0
        .into_iter()
        .flat_map(|batch| batch.items)
        .find(|item| item.id.0.ends_with("control_center.modules"))
        .expect("control_center.modules should be found by its label");
    assert_eq!(item.subtitle, "sill");
}
