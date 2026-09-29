//! The power menu's keyboard and pointer on the real panel in a `ds_native::Harness`, over a
//! dry-run backend (design/20 §1.8: arrows move, Enter confirms, Esc closes; a scrim click
//! closes). A chosen action is only recorded, and only after the exit motion has settled and
//! the menu has closed.

mod power_menu_support;

use std::time::Duration;

use ds::Key;
use power_menu_support::{is_open, look, nth_button, opened, primary, ring_on_primary, settle};
use sill_services::{Offer, Session, SessionAction};
use sill_settings::{PowerMenuDefault, PowerMenuSettings, ShellFile};
use sill_surfaces::surfaces::power_menu::Preselect;

fn file(default_action: PowerMenuDefault) -> ShellFile {
    ShellFile {
        power_menu: PowerMenuSettings {
            default_action,
            ..PowerMenuSettings::default()
        },
        ..ShellFile::default()
    }
}

fn shut_down_default() -> ShellFile {
    file(PowerMenuDefault::ShutDown)
}

#[test]
fn it_opens_on_the_default_button_with_the_ring_on_it() {
    let running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    assert_eq!(primary(&running.harness).as_deref(), Some("Shut Down"));
    assert!(ring_on_primary(&running.harness), "the keyboard is on it");
    assert_eq!(look(&running.harness).as_deref(), Some("open"));
    let suspend = opened(
        file(PowerMenuDefault::Suspend),
        Session::everything(),
        Preselect::Default,
    );
    assert_eq!(primary(&suspend.harness).as_deref(), Some("Suspend"));
}

#[test]
fn a_launcher_row_opens_on_its_own_button() {
    let running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Action(SessionAction::Restart),
    );
    assert_eq!(primary(&running.harness).as_deref(), Some("Restart"));
}

#[test]
fn arrows_and_tab_move_the_highlight_and_the_ring() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    let steps = [
        (Key::Right, "Suspend"),
        (Key::Right, "Log Out"),
        (Key::Left, "Suspend"),
        (Key::Up, "Shut Down"),
        (Key::Tab, "Suspend"),
        (Key::Down, "Log Out"),
        (Key::Right, "Restart"),
    ];
    for (key, want) in steps {
        running.harness.key(key);
        let harness = &mut running.harness;
        settle(harness, |h| {
            primary(h).as_deref() == Some(want) && ring_on_primary(h)
        });
        assert_eq!(primary(harness).as_deref(), Some(want), "after {key:?}");
        assert!(ring_on_primary(harness), "the ring follows to {want}");
    }
    assert!(running.session.ran().is_empty(), "moving runs nothing");
}

#[test]
fn enter_leaves_then_runs_the_focused_action_on_the_backend() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.key(Key::Left);
    settle(&mut running.harness, |h| {
        primary(h).as_deref() == Some("Restart")
    });
    running.harness.key(Key::Enter);
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    assert!(
        running.session.ran().is_empty(),
        "nothing runs while the sheet is still leaving"
    );
    assert!(is_open(&running.link));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link), "the menu closed");
    assert_eq!(running.session.ran(), vec![SessionAction::Restart]);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.session.ran().len(), 1, "run once");
    assert_eq!(look(&running.harness).as_deref(), Some("closed"));
    assert_eq!(
        running.harness.count(".ds-sheet"),
        0,
        "nothing drawn hidden"
    );
}

#[test]
fn escape_leaves_and_closes_running_nothing() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.key(Key::Escape);
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link));
    assert!(running.session.ran().is_empty());
}

#[test]
fn opened_again_while_leaving_it_enters_again_and_stays() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.key(Key::Escape);
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    assert_eq!(
        running
            .harness
            .attr(".ds-sheet", "data-presence")
            .as_deref(),
        Some("leaving")
    );
    running
        .link
        .open(power_menu_support::fake::output().id, Preselect::Default);
    settle(&mut running.harness, |h| look(h).as_deref() == Some("open"));
    assert_eq!(look(&running.harness).as_deref(), Some("open"));
    assert_ne!(
        running
            .harness
            .attr(".ds-sheet", "data-presence")
            .as_deref(),
        Some("leaving"),
        "the entrance plays again"
    );
    running.harness.advance(Duration::from_millis(600));
    assert!(is_open(&running.link), "the exit's settle never closed it");
    assert_eq!(running.harness.count(".ds-sheet"), 1);
    assert!(running.session.ran().is_empty());
}

#[test]
fn a_scrim_click_leaves_and_closes() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.click(ds::Point {
        x: ds::Px(20.0),
        y: ds::Px(780.0),
    });
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link));
    assert!(running.session.ran().is_empty());
}

#[test]
fn a_click_runs_that_button() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    let log_out = running.harness.centre(&nth_button(1));
    assert_eq!(
        running.harness.text_of(&nth_button(1)).as_deref(),
        Some("Log Out")
    );
    running
        .harness
        .click(log_out.expect("the Log Out button is laid out")); // test-only
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert_eq!(running.session.ran(), vec![SessionAction::LogOut]);
}

#[test]
fn unoffered_buttons_are_skipped_disabled_and_inert() {
    let offers = Session {
        suspend: Offer::Unavailable,
        ..Session::everything()
    };
    let mut running = opened(shut_down_default(), offers, Preselect::Default);
    for attr in ["aria-disabled", "disabled"] {
        assert_eq!(
            running.harness.attr(&nth_button(4), attr).as_deref(),
            Some("true"),
            "Suspend's {attr}"
        );
    }
    running.harness.key(Key::Right);
    settle(&mut running.harness, |h| {
        primary(h).as_deref() == Some("Log Out")
    });
    assert_eq!(primary(&running.harness).as_deref(), Some("Log Out"));
    let suspend = running.harness.centre(&nth_button(4));
    running
        .harness
        .click(suspend.expect("the Suspend button is laid out")); // test-only
    running.harness.advance(Duration::from_millis(400));
    assert!(is_open(&running.link), "a disabled button does nothing");
    assert!(running.session.ran().is_empty());
}

#[test]
fn nothing_offered_runs_nothing_on_enter() {
    let mut running = opened(shut_down_default(), Session::default(), Preselect::Default);
    assert_eq!(primary(&running.harness).as_deref(), Some("Shut Down"));
    running.harness.key(Key::Enter);
    running.harness.advance(Duration::from_millis(400));
    assert!(is_open(&running.link));
    assert!(running.session.ran().is_empty());
}
