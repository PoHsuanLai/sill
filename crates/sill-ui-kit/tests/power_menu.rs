//! The power menu on the harness, following a `PowerMenuLink` the test writes as the daemon does,
//! over the fake session service: the offers it publishes are the ones the buttons show, and the
//! `SessionCommand::Run` it is sent is the choice made, so nothing here can reach logind.
//! Hidden it draws nothing; open, quire's Sheet centred over its full-output modal scrim, the
//! question for the focused button, the four buttons in design/20 section 1.8's order and
//! variants at one height, the key hints, and the blur element covering exactly the sheet; then
//! the keyboard and pointer (arrows move, Enter confirms, Esc closes, a scrim click closes). A
//! chosen action is sent only after the exit motion has settled and the menu has closed.

use std::time::Duration;

use ds::ShortcutKey;
use ds_native::{Harness, Viewport};
use sill_model::session::{Offer, Session, SessionAction, SessionCommand};
use sill_settings::{PowerMenuDefault, PowerMenuSettings, ShellFile};
use sill_ui_kit::PowerMenu;
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
    ds_native::harness::settle_until(harness, done);
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

/// The focused button's label: the one drawn `Primary`.
fn primary(harness: &Harness) -> Option<String> {
    harness.text_of(".ds-button[*|data-variant=primary]")
}

/// Whether the keyboard focus is on the `Primary` button (quire's focus ring is there).
fn ring_on_primary(harness: &Harness) -> bool {
    harness.is_focused(".ds-button[*|data-variant=primary]")
}

/// The sheet's phase: its panel's `data-look` (`open` or `leaving`), `closed` with no sheet.
fn look(harness: &Harness) -> Option<String> {
    match harness.count(".sill-power-menu-panel") {
        0 => Some("closed".to_owned()),
        _ => harness.attr(".sill-power-menu-panel", "data-look"),
    }
}

/// Whether the menu is showing.
fn is_open(link: &PowerMenuLink) -> bool {
    link.now().opening().is_some()
}

/// The `n`th button (1-based, `SessionAction::POWER_MENU` order).
fn nth_button(n: usize) -> String {
    format!(".sill-power-menu-buttons .ds-button:nth-child({n})")
}

#[test]
fn hidden_it_draws_nothing() {
    let mut running = start(ShellFile::default(), Session::everything());
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(look(&running.harness).as_deref(), Some("closed"));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".ds-scrim"), 0);
    assert_eq!(running.harness.count(".ds-button"), 0);
}

#[test]
fn open_it_draws_the_sheet_the_buttons_and_the_hints() {
    let running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.count(".ds-scrim"), 1);
    assert_eq!(
        h.attr(".ds-scrim", "data-strength").as_deref(),
        Some("modal")
    );
    assert_eq!(
        h.attr(".ds-sheet", "data-placement").as_deref(),
        Some("centre")
    );
    assert_eq!(
        h.text_of(".sill-power-menu-prompt").as_deref(),
        Some("Shut down this computer now?")
    );
    let buttons = [
        ("Log Out", "secondary", None),
        ("Restart", "danger", Some("regular")),
        ("Shut Down", "primary", None),
        ("Suspend", "secondary", None),
    ];
    assert_eq!(
        h.count(".sill-power-menu-buttons .ds-button"),
        buttons.len()
    );
    for (n, (label, variant, size)) in buttons.iter().enumerate() {
        let button = nth_button(n + 1);
        assert_eq!(h.text_of(&button).as_deref(), Some(*label));
        assert_eq!(
            h.attr(&button, "data-variant").as_deref(),
            Some(*variant),
            "{label}"
        );
        assert_eq!(h.attr(&button, "data-size").as_deref(), *size, "{label}");
    }
    // Restart (Danger, Regular) stands level with Log Out and Suspend (Secondary); Shut Down
    // (Primary) is their box less the Secondary's two hairlines, quire's own geometry.
    let heights: Vec<f32> = [1, 2, 4]
        .into_iter()
        .filter_map(|n| h.rect(&nth_button(n)).map(|r| r.size.height.0))
        .collect();
    assert_eq!(heights.len(), 3);
    assert!(
        heights
            .iter()
            .all(|height| (height - heights[0]).abs() < 0.5),
        "Danger stands level with Secondary: {heights:?}"
    );
    assert_eq!(h.count(".sill-power-menu-hint"), 3);
    assert_eq!(h.count(".sill-power-menu-hint .ds-kbd"), 4, "← →, ↵, Esc");
}

#[test]
fn the_scrim_covers_the_output_and_the_blur_element_is_the_sheet() {
    let running = opened(
        ShellFile::default(),
        Session::everything(),
        Preselect::Default,
    );
    let h = &running.harness;
    let scrim = h.rect(".ds-scrim").expect("the scrim is laid out"); // test-only
    assert_eq!(
        (scrim.size.width.0, scrim.size.height.0),
        (WIDTH, HEIGHT),
        "the whole output dims"
    );
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
        (ShortcutKey::Right, "Suspend"),
        (ShortcutKey::Right, "Log Out"),
        (ShortcutKey::Left, "Suspend"),
        (ShortcutKey::Up, "Shut Down"),
        (ShortcutKey::Tab, "Suspend"),
        (ShortcutKey::Down, "Log Out"),
        (ShortcutKey::Right, "Restart"),
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
    assert!(ran(&mut running).is_empty(), "moving runs nothing");
}

#[test]
fn enter_leaves_then_runs_the_focused_action_on_the_backend() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.key(ShortcutKey::Left);
    settle(&mut running.harness, |h| {
        primary(h).as_deref() == Some("Restart")
    });
    running.harness.key(ShortcutKey::Enter);
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
fn escape_leaves_and_closes_running_nothing() {
    let mut running = opened(
        shut_down_default(),
        Session::everything(),
        Preselect::Default,
    );
    running.harness.key(ShortcutKey::Escape);
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
    running.harness.key(ShortcutKey::Escape);
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
    assert!(ran(&mut running).is_empty());
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
    assert!(ran(&mut running).is_empty());
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
    assert_eq!(ran(&mut running), vec![SessionAction::LogOut]);
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
    running.harness.key(ShortcutKey::Right);
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
    assert!(ran(&mut running).is_empty());
}

#[test]
fn nothing_offered_runs_nothing_on_enter() {
    let mut running = opened(shut_down_default(), Session::default(), Preselect::Default);
    assert_eq!(primary(&running.harness).as_deref(), Some("Shut Down"));
    running.harness.key(ShortcutKey::Enter);
    running.harness.advance(Duration::from_millis(400));
    assert!(is_open(&running.link));
    assert!(ran(&mut running).is_empty());
}

#[test]
fn the_stylesheet_names_no_keyframes_of_its_own() {
    assert!(
        !sill_ui_kit::surfaces::power_menu::POWER_MENU_CSS.contains("@keyframes"),
        "motion is quire's (coherence rule 4)"
    );
}
