//! The launcher's keyboard map (design/06 §20.2) on the real panel in a `ds_harness::Harness`,
//! over the fake search runner: Down, Down, Enter runs the third row; Tab moves to the next
//! provider's group; Escape closes.
//!
//! "fi" finds, best first: Firefox (the top hit), Files (Applications), "Firefox — sill
//! launcher" (Windows), Wi-Fi (Settings).

mod launcher_harness;

use ds::ShortcutKey;
use launcher_harness::launcher_support::{self, FIREFOX_WINDOW};
use launcher_harness::{is_open, open, settle, titles};
use sill_launcher::WindowKey;
use sill_model::cosmic_wl::WlCommand;
use sill_model::results::SearchCommand;

#[test]
fn down_down_enter_runs_the_third_row() {
    let mut open = open("fi");
    assert_eq!(
        titles(&open.harness),
        vec!["Firefox", "Files", "Firefox — sill launcher", "Wi-Fi"]
    );
    open.harness.key(ShortcutKey::Down);
    open.harness.key(ShortcutKey::Down);
    open.harness.key(ShortcutKey::Enter);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    assert!(!is_open(&open.link), "Enter closes the launcher");
    let sent = open.feeds.bar.wl.try_recv();
    assert_eq!(
        sent.ok(),
        Some(WlCommand::Activate(WindowKey(FIREFOX_WINDOW))),
        "the third row is the Firefox window"
    );
    let chosen = open
        .feeds
        .searches
        .lock()
        .map(|log| log.clone())
        .unwrap_or_default();
    assert!(
        chosen
            .iter()
            .any(|c| matches!(c, SearchCommand::Chosen { key, .. } if key.0 == "windows:10")),
        "the choice is counted for frecency: {chosen:?}"
    );
}

#[test]
fn tab_moves_to_the_next_group() {
    let mut open = open("fi");
    open.harness.key(ShortcutKey::Tab);
    settle(&mut open.harness, |h| h.count(".ds-row") == 2);
    assert_eq!(
        titles(&open.harness),
        vec!["Firefox", "Files"],
        "Applications only"
    );
    open.harness.key(ShortcutKey::Tab);
    settle(&mut open.harness, |h| h.count(".ds-row") == 1);
    assert_eq!(
        titles(&open.harness),
        vec!["Firefox — sill launcher"],
        "then Windows"
    );
    // The field still has the keyboard after the palette remounted: Enter runs the window.
    open.harness.key(ShortcutKey::Enter);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    assert_eq!(
        open.feeds.bar.wl.try_recv().ok(),
        Some(WlCommand::Activate(WindowKey(FIREFOX_WINDOW)))
    );
}

#[test]
fn escape_closes() {
    let mut open = open("fi");
    assert!(is_open(&open.link));
    open.harness.key(ShortcutKey::Escape);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    assert!(!is_open(&open.link));
    assert!(open.feeds.bar.wl.try_recv().is_err(), "nothing ran");
    assert!(
        open.feeds.bar.launch.try_recv().is_err(),
        "nothing launched"
    );
}

#[test]
fn enter_on_the_top_hit_launches_it() {
    let mut open = open("fire");
    open.harness.key(ShortcutKey::Enter);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    match open.feeds.bar.launch.try_recv() {
        Ok(sill_model::launch::LaunchCommand::Launch(request)) => {
            assert_eq!(request.app.0, launcher_support::FIREFOX);
        }
        other => panic!("no launch: {other:?}"),
    }
}
