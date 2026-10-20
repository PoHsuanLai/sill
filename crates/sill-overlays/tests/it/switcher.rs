//! The switcher on the harness, on the virtual clock and fake services (design/13
//! §13.3.5): a chord shows quire's `AppSwitcher` after the show delay with the apps in MRU
//! order and the previous one selected; a commit inside the delay switches with no panel; Tab,
//! Shift+Tab and the chord step; the release (Alt's key-up) and a click activate; Escape and
//! cancel touch nothing; Q closes the app's windows and folds its tile; H minimizes; the
//! made-up apps of `sill debug switcher` show at once and touch no window.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{FocusState, Harness, Input, Viewport};
use sill_launcher::{AppId, WindowKey};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};
use sill_overlays::Switcher;
use sill_settings::ShellFile;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{Dir, SwitcherAsk, SwitcherLink};

type Running = SurfaceHarness<Switcher>;

/// Ask the switcher on the test output, taken at this instant: the next `advance` runs the
/// woken switcher at `harness.now()` before it steps any timer.
fn ask(running: &mut Running, ask: SwitcherAsk) {
    running.link.ask(fake::output().id, ask);
}

/// The commands sent so far, drained.
fn sent(running: &mut Running) -> Vec<WlCommand> {
    std::iter::from_fn(|| running.feeds.wl.try_recv().ok()).collect()
}

/// Windows of `apps` (app id, window key), most recently activated first.
fn windows(apps: &[(&str, u64)]) -> Toplevels {
    let window = |(app_id, key): &(&str, u64)| Toplevel {
        identifier: String::new(),
        key: WindowKey(*key),
        app_id: (*app_id).to_owned(),
        title: format!("{app_id} {key}"),
        activity: Activity::Inactive,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: vec![WorkspaceKey(2)],
        outputs: vec![OutputName("DP-1".to_owned())],
    };
    Toplevels {
        windows: apps.iter().map(window).collect(),
    }
}

/// The switcher on `file`'s settings over `toplevels`, nothing shown yet.
fn start(file: ShellFile, toplevels: Toplevels) -> Running {
    let running = SurfaceHarness::<Switcher>::setup(SwitcherLink::new(), ())
        .settings(file)
        .viewport(Viewport {
            width: 1280,
            height: 720,
            scale_percent: 100,
        })
        .start();
    running
        .feeds
        .windows
        .send_modify(|wl| wl.toplevels = toplevels);
    running
}

/// The tiles' names in order (`aria-label`), and the selected one's.
fn tiles(harness: &Harness) -> (Vec<String>, Option<String>) {
    let html = harness.html();
    let mut names = Vec::new();
    let mut selected = None;
    let mut rest = html.as_str();
    while let Some(at) = rest.find("role=\"option\"") {
        rest = &rest[at..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        let name = attr(tag, "aria-label").unwrap_or_default();
        if attr(tag, "aria-selected").as_deref() == Some("true") {
            selected = Some(name.clone());
        }
        names.push(name);
        rest = &rest["role=\"option\"".len()..];
    }
    (names, selected)
}

/// A tile's `data-presence` (`leaving` while it folds).
fn presence(harness: &Harness, name: &str) -> Option<String> {
    let html = harness.html();
    let needle = format!("aria-label=\"{name}\"");
    let at = html.find(&needle)?;
    let start = html[..at].rfind('<')?;
    let tag = &html[start..at + html[at..].find('>')?];
    attr(tag, "data-presence")
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let rest = &tag[tag.find(&needle)? + needle.len()..];
    Some(rest[..rest.find('"')?].to_owned())
}

/// Three apps, `term` most recent; `calc` has two windows.
fn running() -> Running {
    start(
        ShellFile::default(),
        windows(&[("term", 1), ("calc", 2), ("edit", 3), ("calc", 4)]),
    )
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|&s| s.to_owned()).collect()
}

fn wait(running: &mut Running, ms: u64) {
    running.harness.advance(Duration::from_millis(ms));
}

fn panels(running: &Running) -> usize {
    running.harness.count(".ds-switcher")
}

/// Chord, then past the show delay (150 ms).
fn shown() -> Running {
    let mut running = running();
    wait(&mut running, 40);
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 260);
    running
}

#[test]
fn a_chord_shows_the_apps_after_the_delay_with_the_previous_selected() {
    let mut running = running();
    wait(&mut running, 40);
    assert_eq!(panels(&running), 0, "hidden at start");
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    let delay = u64::from(ShellFile::default().switcher.show_delay_ms.0);
    wait(&mut running, delay - 1);
    assert_eq!(
        panels(&running),
        0,
        "not a millisecond before the show delay"
    );
    wait(&mut running, 1);
    assert_eq!(panels(&running), 1, "shown at it");
    wait(&mut running, 110);
    assert_eq!(
        tiles(&running.harness),
        (names(&["Term", "Calc", "Edit"]), Some("Calc".to_owned()))
    );
    assert_eq!(
        running.harness.focus_of(".sill-switcher"),
        FocusState::Focused,
        "the key target has the focus"
    );
    assert_eq!(sent(&mut running), vec![]);
}

#[test]
fn a_commit_inside_the_delay_switches_with_no_panel() {
    let mut running = running();
    wait(&mut running, 40);
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 30);
    ask(&mut running, SwitcherAsk::Commit);
    for _ in 0..10 {
        wait(&mut running, 30);
        assert_eq!(panels(&running), 0, "no panel at any point");
    }
    assert_eq!(sent(&mut running), vec![WlCommand::Activate(WindowKey(2))]);
}

/// Before the modifiers are read (the focus lands 50 to 90 ms after the chord), a second chord
/// is Tab pressed again with Cmd still down: no one releases and presses the chord again that
/// fast. It steps; the release then activates the third app.
#[test]
fn a_second_chord_before_the_modifiers_are_read_is_a_step() {
    let mut running = running();
    wait(&mut running, 40);
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 30);
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 300);
    assert_eq!(panels(&running), 1);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("Edit"));
    ask(&mut running, SwitcherAsk::Commit);
    wait(&mut running, 60);
    assert_eq!(panels(&running), 0);
    assert_eq!(sent(&mut running), vec![WlCommand::Activate(WindowKey(3))]);
}

#[test]
fn keys_and_chords_step_and_the_release_activates() {
    let mut running = shown();
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Tab));
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("Edit"));
    running.harness.send(Input::chord(
        &[ShortcutKey::Alt, ShortcutKey::Shift],
        ShortcutKey::Tab,
    ));
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Char('`')));
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("Term"));
    ask(&mut running, SwitcherAsk::Chord(Dir::Prev));
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("Edit"));
    assert_eq!(sent(&mut running), vec![], "stepping touches no window");
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Alt));
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0, "closed at the release");
    assert_eq!(sent(&mut running), vec![WlCommand::Activate(WindowKey(3))]);
}

#[test]
fn escape_and_cancel_close_touching_nothing() {
    let mut running = shown();
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Escape));
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    ask(&mut running, SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 260);
    assert_eq!(panels(&running), 1);
    ask(&mut running, SwitcherAsk::Cancel);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Alt));
    wait(&mut running, 20);
    assert_eq!(sent(&mut running), vec![]);
}

#[test]
fn q_quits_the_selected_app_and_its_tile_folds_away() {
    let mut running = shown();
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Char('q')));
    wait(&mut running, 20);
    assert_eq!(
        sent(&mut running),
        vec![
            WlCommand::Close(WindowKey(2)),
            WlCommand::Close(WindowKey(4))
        ]
    );
    assert_eq!(
        presence(&running.harness, "Calc").as_deref(),
        Some("leaving")
    );
    wait(&mut running, 700);
    assert_eq!(panels(&running), 1, "the panel stays");
    assert_eq!(
        tiles(&running.harness),
        (names(&["Term", "Edit"]), Some("Edit".to_owned())),
        "the tile went; the selection stays in place"
    );
}

#[test]
fn h_hides_the_selected_app_and_stays() {
    let mut running = shown();
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Char('h')));
    wait(&mut running, 20);
    assert_eq!(
        sent(&mut running),
        vec![
            WlCommand::Minimize(WindowKey(2)),
            WlCommand::Minimize(WindowKey(4))
        ]
    );
    assert_eq!(panels(&running), 1);
}

#[test]
fn hover_selects_and_a_click_activates() {
    let mut running = shown();
    // The third cell: the first's rect, two cells and gaps (8) along.
    let first = running
        .harness
        .rect(".ds-switcher-cell")
        .expect("the tiles are laid out"); // test-only
    let at = Point {
        x: Px(first.origin.x.0 + 2.0 * (first.size.width.0 + 8.0) + first.size.width.0 / 2.0),
        y: Px(first.origin.y.0 + first.size.height.0 / 2.0),
    };
    // The pointer's first sighting is where it rests; the tile is chosen once it moves.
    let resting = Point {
        x: Px(at.x.0 - 10.0),
        y: at.y,
    };
    running.harness.send(Input::pointer_move(resting));
    wait(&mut running, 20);
    let before = tiles(&running.harness).1;
    assert_ne!(
        before.as_deref(),
        Some("Edit"),
        "a tile under a resting pointer is not chosen"
    );
    running.harness.send(Input::pointer_move(at));
    wait(&mut running, 20);
    assert_eq!(
        tiles(&running.harness).1.as_deref(),
        Some("Edit"),
        "hover selects once the pointer moves"
    );
    running.harness.send(Input::click(at));
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    assert_eq!(sent(&mut running), vec![WlCommand::Activate(WindowKey(3))]);
}

#[test]
fn made_up_apps_show_at_once_and_touch_no_window() {
    let mut running = running();
    wait(&mut running, 40);
    let apps = ["files", "mail", "notes"]
        .map(|a| AppId(a.to_owned()))
        .to_vec();
    ask(&mut running, SwitcherAsk::Fake(apps));
    // Shown at the ask, and the key target's `focus_soon` lands in the same frame, every run:
    // no `FRAME_SLACK` allowance.
    wait(&mut running, 0);
    assert_eq!(
        tiles(&running.harness),
        (names(&["Files", "Mail", "Notes"]), Some("Mail".to_owned()))
    );
    assert_eq!(
        running.harness.focus_of(".sill-switcher"),
        FocusState::Focused,
        "focused at the ask"
    );
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Char('q')));
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Alt], ShortcutKey::Alt));
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    assert_eq!(sent(&mut running), vec![]);
}
