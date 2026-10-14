//! The power menu on the harness, following a `PowerMenuLink` the test writes as the daemon does,
//! over the fake session service: the offers it publishes are the ones the buttons show, and the
//! `SessionCommand::Run` it is sent is the choice made, so nothing here can reach logind.
//! Hidden it draws nothing; open, quire's Alert on a Sheet centred full-output (it dims nothing),
//! the question for the opening's action, its buttons (that action first and the default, then the
//! other offered ones, then Cancel) and the blur element covering exactly the sheet; then the
//! keyboard and pointer (Return presses the default, Tab moves, Space presses, Escape cancels, a
//! press outside the sheet closes). A chosen action is sent only after the exit motion has settled
//! and the menu has closed.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Driver, FocusState, Harness, Input, Query, Viewport};
use sill_model::session::{Offer, Session, SessionAction, SessionCommand};
use sill_overlays::PowerMenu;
use sill_settings::{PowerMenuDefault, PowerMenuSettings, ShellFile};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{PowerMenuLink, Preselect};

type Running = SurfaceHarness<PowerMenu>;

/// The viewport's size, as the rects report it.
const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;

/// The viewport: a small output.
const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// The menu on `file`'s settings with `offers`, hidden.
fn start(file: ShellFile, offers: Session) -> Running {
    let run = SurfaceHarness::<PowerMenu>::setup(PowerMenuLink::new(), ())
        .settings(file)
        .viewport(VIEWPORT)
        .start();
    run.feeds.session.send_replace(offers);
    run
}

/// The menu opened with `preselect`, once its sheet has settled.
fn opened(file: ShellFile, offers: Session, preselect: Preselect) -> Running {
    let mut running = start(file, offers);
    running.harness.advance(Duration::from_millis(40));
    running.link.open(fake::output().id, preselect);
    settle(&mut running.harness, |h| {
        h.count(".ds-sheet") == 1 && ring_on_primary(h)
    });
    running.harness.advance(Duration::from_millis(600));
    running
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The actions the menu has asked the session service to run since the last call.
fn ran(running: &mut Running) -> Vec<SessionAction> {
    let mut ran = Vec::new();
    while let Ok(command) = running.feeds.session_commands.try_recv() {
        if let SessionCommand::Run(action) = command {
            ran.push(action);
        }
    }
    ran
}

/// The focused button's label: the one that answers Return.
fn primary(harness: &Harness) -> Option<String> {
    harness.text_of(".ds-button[*|data-answers=return]")
}

/// Whether the keyboard focus is on the default button (quire's focus ring is there).
fn ring_on_primary(harness: &Harness) -> bool {
    harness.focus_of(".ds-button[*|data-answers=return]") == FocusState::Focused
}

/// The sheet's phase: its `data-presence` (`entering`, `present` or `leaving`), `closed` with no
/// sheet.
fn look(harness: &Harness) -> Option<String> {
    match harness.count(".ds-sheet") {
        0 => Some("closed".to_owned()),
        _ => harness.attr(".ds-sheet", "data-presence"),
    }
}

/// Whether the menu is showing.
fn is_open(link: &PowerMenuLink) -> bool {
    link.now().opening().is_some()
}

/// The `n`th button of the alert (1-based, top to bottom).
fn nth_button(n: usize) -> String {
    format!(".ds-alert-slot:nth-child({n}) .ds-button")
}

#[test]
fn hidden_it_draws_nothing() {
    let mut running = start(ShellFile::default(), Session::everything());
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(look(&running.harness).as_deref(), Some("closed"));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".ds-button"), 0);
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn open_it_draws_the_alert_and_its_buttons() {
    let running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.count(".ds-scrim"), 0, "a sheet dims nothing");
    assert_eq!(
        h.attr(".ds-sheet", "data-attach").as_deref(),
        Some("centre")
    );
    assert_eq!(
        h.text_of(".ds-alert-title").as_deref(),
        Some("Shut down this computer now?")
    );
    assert_eq!(
        h.text_of(".ds-alert-body").as_deref(),
        Some("Apps quit and the computer turns off.")
    );
    assert_eq!(
        h.attr(".ds-alert-footer", "data-layout").as_deref(),
        Some("stack"),
        "five buttons stack, the default on top"
    );
    let buttons = [
        ("Shut Down", "normal", Some("return")),
        ("Log Out", "normal", None),
        ("Restart", "destructive", None),
        ("Suspend", "normal", None),
        ("Cancel", "normal", Some("escape")),
    ];
    assert_eq!(h.count(".ds-alert-footer .ds-button"), buttons.len());
    for (n, (label, role, answers)) in buttons.iter().enumerate() {
        let button = nth_button(n + 1);
        assert_eq!(h.text_of(&button).as_deref(), Some(*label));
        assert_eq!(
            h.attr(&button, "data-role").as_deref(),
            Some(*role),
            "{label}"
        );
        assert_eq!(
            h.attr(&button, "data-answers").as_deref(),
            *answers,
            "{label}"
        );
    }
    assert_eq!(
        h.count(".ds-key-equivalent"),
        0,
        "no key hints: the alert's buttons say what Return and Escape do"
    );
}

#[test]
fn the_blur_element_is_the_sheet() {
    let running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let h = &running.harness;
    let sheet = h.rect(".ds-sheet").expect("the sheet is laid out"); // test-only
    let panel = h
        .rect("#power-menu-panel")
        .expect("the blur element is laid out"); // test-only
    assert_eq!(panel, sheet, "the blur region is the sheet's box");
    let middle = sheet.origin.x.0 + sheet.size.width.0 / 2.0;
    assert!(
        (middle - WIDTH / 2.0).abs() < 1.0,
        "centred across the output: {sheet:?}"
    );
    let centre = sheet.origin.y.0 + sheet.size.height.0 / 2.0;
    assert!(
        (centre - HEIGHT / 2.0).abs() < 1.0,
        "centred down the output: {sheet:?}"
    );
    let root = h.rect(".ds").expect("the root is laid out"); // test-only
    assert_eq!(
        (root.size.width.0, root.size.height.0),
        (WIDTH, HEIGHT),
        "the root is the viewport"
    );
    assert!(sheet.size.height.0 > 100.0, "{sheet:?}");
}

/// A picture of the open menu for eyes, under the test target's scratch directory.
#[test]
fn the_open_menu_renders() {
    let mut running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let image = running
        .harness
        .render()
        .expect("the harness renders the menu"); // test-only
    assert_eq!(
        (image.width(), image.height()),
        (VIEWPORT.width, VIEWPORT.height)
    );
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("power-menu-harness.png");
    image.save(&path).expect("the picture saves"); // test-only
}

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
    assert_eq!(look(&running.harness).as_deref(), Some("present"));
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

/// The label of the button that holds the keyboard.
fn focused(harness: &Harness) -> Option<String> {
    (1..=6)
        .map(nth_button)
        .find(|button| harness.focus_of(button) == FocusState::Focused)
        .and_then(|button| harness.text_of(&button))
}

#[test]
fn tab_and_shift_tab_move_the_ring_between_the_buttons_and_run_nothing() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    let steps = [(ShortcutKey::Tab, "Log Out"), (ShortcutKey::Tab, "Restart")];
    for (key, want) in steps {
        running.harness.send(Input::key(key));
        let harness = &mut running.harness;
        settle(harness, |h| focused(h).as_deref() == Some(want));
        assert_eq!(focused(harness).as_deref(), Some(want), "after {key:?}");
    }
    assert!(ran(&mut running).is_empty(), "moving runs nothing");
}

#[test]
fn return_leaves_then_runs_the_default_action_on_the_backend() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Action(SessionAction::Restart),
    );
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    assert!(
        ran(&mut running).is_empty(),
        "nothing runs while the sheet is still leaving"
    );
    assert!(is_open(&running.link));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link), "the menu closed");
    assert_eq!(ran(&mut running), vec![SessionAction::Restart]);
    running.harness.advance(Duration::from_millis(100));
    assert!(ran(&mut running).is_empty(), "run once");
    assert_eq!(look(&running.harness).as_deref(), Some("closed"));
    assert_eq!(
        running.harness.count(".ds-sheet"),
        0,
        "nothing drawn hidden"
    );
}

#[test]
fn space_presses_the_button_the_keyboard_is_on() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.send(Input::key(ShortcutKey::Tab));
    settle(&mut running.harness, |h| {
        focused(h).as_deref() == Some("Log Out")
    });
    running.harness.send(Input::key(ShortcutKey::Space));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert_eq!(ran(&mut running), vec![SessionAction::LogOut]);
}

#[test]
fn escape_leaves_and_closes_running_nothing() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link));
    assert!(ran(&mut running).is_empty());
}

#[test]
fn opened_again_while_leaving_it_enters_again_and_stays() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    assert_eq!(
        running
            .harness
            .attr(".ds-sheet", "data-presence")
            .as_deref(),
        Some("leaving")
    );
    running.link.open(fake::output().id, Preselect::Default);
    settle(&mut running.harness, |h| {
        look(h).as_deref() == Some("present")
    });
    assert_eq!(look(&running.harness).as_deref(), Some("present"));
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
    assert!(ran(&mut running).is_empty());
}

#[test]
fn a_press_outside_the_sheet_leaves_and_closes() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.send(Input::click(Point {
        x: Px(20.0),
        y: Px(780.0),
    }));
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(look(&running.harness).as_deref(), Some("leaving"));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(!is_open(&running.link));
    assert!(ran(&mut running).is_empty());
}

#[test]
fn a_click_runs_that_button() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    let log_out = running.harness.centre(&nth_button(2));
    assert_eq!(
        running.harness.text_of(&nth_button(2)).as_deref(),
        Some("Log Out")
    );
    running.harness.send(Input::click(
        log_out.expect("the Log Out button is laid out"),
    )); // test-only
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert_eq!(ran(&mut running), vec![SessionAction::LogOut]);
}

#[test]
fn cancel_leaves_and_closes_running_nothing() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    let cancel = running.harness.centre(&nth_button(5));
    running
        .harness
        .send(Input::click(cancel.expect("the Cancel button is laid out"))); // test-only
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(ran(&mut running).is_empty());
}

#[test]
fn an_action_logind_does_not_offer_is_left_out() {
    let offers = Session {
        suspend: Offer::Unavailable,
        ..Session::everything()
    };
    let running = opened(shut_down_default(), offers, Preselect::Default);
    let h = &running.harness;
    let labels: Vec<Option<String>> = (1..=4).map(|n| h.text_of(&nth_button(n))).collect();
    assert_eq!(
        labels,
        ["Shut Down", "Log Out", "Restart", "Cancel"].map(|label| Some(label.to_owned()))
    );
    assert_eq!(h.count(".ds-alert-footer .ds-button"), 4);
}

#[test]
fn with_nothing_offered_the_alert_only_cancels() {
    let mut running = opened(shut_down_default(), Session::default(), Preselect::Default);
    assert_eq!(
        running.harness.count(".ds-alert-footer .ds-button"),
        1,
        "only Cancel"
    );
    running.harness.send(Input::key(ShortcutKey::Enter));
    let link = running.link.clone();
    settle(&mut running.harness, |_| !is_open(&link));
    assert!(ran(&mut running).is_empty());
}
