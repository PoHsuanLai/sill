use sill_launcher::{AppId, ItemId, ItemKey, ProviderKind, UnixTime};

use super::{ModelOffered, Store, first_apps, first_dock, first_frecency, first_model_offer};
use crate::apps::{AppIndex, AppsEvent, Listing, step, test_app};
use crate::launch::assistant_model_setup;
use crate::pins::DockPins;

const NOW: UnixTime = UnixTime(1_790_000_000);

fn installed(entries: &[(&str, &[&str], Listing)]) -> AppIndex {
    let apps = entries
        .iter()
        .map(|(id, categories, listing)| {
            let mut app = test_app(id, id);
            app.categories = categories.iter().map(|c| (*c).to_owned()).collect();
            app.listing = *listing;
            app
        })
        .collect();
    step(AppIndex::default(), AppsEvent::Scanned(apps))
}

fn ids(apps: Vec<AppId>) -> Vec<String> {
    apps.into_iter().map(|app| app.0).collect()
}

const SHOWN: Listing = Listing::Shown;
const NO_CATEGORIES: &[&str] = &[];
const TERMINAL: &[&str] = &["System", "TerminalEmulator"];

#[test]
fn the_desktops_own_apps_come_in_the_macs_order() {
    type Case = (&'static str, AppIndex, &'static [&'static str]);
    let cases: Vec<Case> = vec![
        (
            "everything installed",
            installed(&[
                ("org.quire.Settings", NO_CATEGORIES, SHOWN),
                ("foot", TERMINAL, SHOWN),
                ("org.quire.Anyview", NO_CATEGORIES, SHOWN),
                ("mailo", NO_CATEGORIES, SHOWN),
                ("org.mozilla.firefox", NO_CATEGORIES, SHOWN),
            ]),
            &["mailo", "org.quire.Anyview", "foot", "org.quire.Settings"],
        ),
        (
            "a missing app leaves a gap, not a hole",
            installed(&[
                ("org.quire.Settings", NO_CATEGORIES, SHOWN),
                ("mailo", NO_CATEGORIES, SHOWN),
            ]),
            &["mailo", "org.quire.Settings"],
        ),
        (
            "the first terminal by id",
            installed(&[("kitty", TERMINAL, SHOWN), ("foot", TERMINAL, SHOWN)]),
            &["foot"],
        ),
        (
            "a hidden entry is not pinned",
            installed(&[
                ("mailo", NO_CATEGORIES, Listing::Hidden),
                ("foot", TERMINAL, Listing::Hidden),
                ("org.quire.Settings", NO_CATEGORIES, SHOWN),
            ]),
            &["org.quire.Settings"],
        ),
        (
            "other apps are not the desktop's",
            installed(&[("org.mozilla.firefox", NO_CATEGORIES, SHOWN)]),
            &[],
        ),
        ("nothing installed", AppIndex::default(), &[]),
    ];
    for (name, index, want) in cases {
        assert_eq!(ids(first_apps(&index)), want, "{name}");
    }
}

#[test]
fn the_dock_is_seeded_once_and_only_when_something_is_installed() {
    let some = installed(&[
        ("mailo", NO_CATEGORIES, SHOWN),
        ("org.quire.Settings", NO_CATEGORIES, SHOWN),
    ]);
    let seeded = DockPins {
        pinned: vec![
            AppId("mailo".to_owned()),
            AppId("org.quire.Settings".to_owned()),
        ],
    };
    let cases = [
        ("a fresh account", Store::Missing, &some, Some(seeded)),
        ("a dock the person emptied", Store::Present, &some, None),
        (
            "a fresh account with none installed",
            Store::Missing,
            &AppIndex::default(),
            None,
        ),
    ];
    for (name, store, index, want) in cases {
        assert_eq!(first_dock(store, index), want, "{name}");
    }
}

#[test]
fn the_launchers_recent_row_is_seeded_with_the_first_app_on_top() {
    let index = installed(&[
        ("mailo", NO_CATEGORIES, SHOWN),
        ("org.quire.Anyview", NO_CATEGORIES, SHOWN),
        ("org.quire.Settings", NO_CATEGORIES, SHOWN),
    ]);
    assert_eq!(first_frecency(Store::Present, &index, NOW), None);
    assert_eq!(
        first_frecency(Store::Missing, &AppIndex::default(), NOW),
        None
    );
    let seeded = first_frecency(Store::Missing, &index, NOW).expect("a fresh account is seeded"); // test-only
    let key = |id: &str| ItemKey::of(ProviderKind::Apps, &ItemId(id.to_owned()));
    let scores: Vec<u16> = ["mailo", "org.quire.Anyview", "org.quire.Settings"]
        .into_iter()
        .map(|id| seeded.score(&key(id), NOW).0)
        .collect();
    assert_eq!(scores, [300, 200, 100]);
}

#[test]
fn the_first_start_offers_the_first_run_picker_once() {
    let cases = [
        (
            "never offered",
            Store::Missing,
            Some(assistant_model_setup()),
        ),
        ("already offered", Store::Present, None),
    ];
    for (name, record, want) in cases {
        assert_eq!(first_model_offer(record), want, "{name}");
    }
}

#[test]
fn the_record_of_the_offer_round_trips() {
    let record = ModelOffered {
        page: assistant_model_setup(),
    };
    let text = serde_json::to_string(&record).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(text, r#"{"page":"page:assistant-model-setup"}"#);
    let back: ModelOffered = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, record);
}
