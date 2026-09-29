//! The real `SwitcherPanel` in a Blitz harness, on the virtual clock and fake services (design/13
//! §13.3.5): a chord shows quire's `AppSwitcher` after the show delay with the apps in MRU
//! order and the previous one selected; a commit inside the delay switches with no panel; Tab,
//! Shift+Tab and the chord step; the release (Alt's key-up) and a click activate; Escape and
//! cancel touch nothing; Q closes the app's windows and folds its tile; H minimizes; the
//! made-up apps of `sill debug switcher` show at once and touch no window.

mod switcher_support;

use std::time::Duration;

use ds::ShortcutKey;
use sill_launcher::{AppId, WindowKey};
use sill_model::cosmic_wl::WlCommand;
use sill_settings::ShellFile;
use sill_ui_kit::links::{Dir, SwitcherAsk};
use switcher_support::{Running, presence, start, tiles, windows};

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
    running.ask(SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 260);
    running
}

#[test]
fn a_chord_shows_the_apps_after_the_delay_with_the_previous_selected() {
    let mut running = running();
    wait(&mut running, 40);
    assert_eq!(panels(&running), 0, "hidden at start");
    running.ask(SwitcherAsk::Chord(Dir::Next));
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
        (names(&["term", "calc", "edit"]), Some("calc".to_owned()))
    );
    assert!(
        running.harness.is_focused(".sill-switcher"),
        "the key target has the focus"
    );
    assert_eq!(running.sent(), vec![]);
}

#[test]
fn a_commit_inside_the_delay_switches_with_no_panel() {
    let mut running = running();
    wait(&mut running, 40);
    running.ask(SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 30);
    running.ask(SwitcherAsk::Commit);
    for _ in 0..10 {
        wait(&mut running, 30);
        assert_eq!(panels(&running), 0, "no panel at any point");
    }
    assert_eq!(running.sent(), vec![WlCommand::Activate(WindowKey(2))]);
}

#[test]
fn a_second_chord_inside_the_delay_is_a_second_quick_tap() {
    let mut running = running();
    wait(&mut running, 40);
    running.ask(SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 30);
    running.ask(SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 300);
    assert_eq!(panels(&running), 0);
    assert_eq!(running.sent(), vec![WlCommand::Activate(WindowKey(2))]);
}

#[test]
fn keys_and_chords_step_and_the_release_activates() {
    let mut running = shown();
    running.harness.chord(&[ShortcutKey::Alt], ShortcutKey::Tab);
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("edit"));
    running
        .harness
        .chord(&[ShortcutKey::Alt, ShortcutKey::Shift], ShortcutKey::Tab);
    running
        .harness
        .chord(&[ShortcutKey::Alt], ShortcutKey::Char('`'));
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("term"));
    running.ask(SwitcherAsk::Chord(Dir::Prev));
    wait(&mut running, 20);
    assert_eq!(tiles(&running.harness).1.as_deref(), Some("edit"));
    assert_eq!(running.sent(), vec![], "stepping touches no window");
    running.harness.chord(&[ShortcutKey::Alt], ShortcutKey::Alt);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0, "closed at the release");
    assert_eq!(running.sent(), vec![WlCommand::Activate(WindowKey(3))]);
}

#[test]
fn escape_and_cancel_close_touching_nothing() {
    let mut running = shown();
    running
        .harness
        .chord(&[ShortcutKey::Alt], ShortcutKey::Escape);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    running.ask(SwitcherAsk::Chord(Dir::Next));
    wait(&mut running, 260);
    assert_eq!(panels(&running), 1);
    running.ask(SwitcherAsk::Cancel);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    running.harness.chord(&[ShortcutKey::Alt], ShortcutKey::Alt);
    wait(&mut running, 20);
    assert_eq!(running.sent(), vec![]);
}

#[test]
fn q_quits_the_selected_app_and_its_tile_folds_away() {
    let mut running = shown();
    running
        .harness
        .chord(&[ShortcutKey::Alt], ShortcutKey::Char('q'));
    wait(&mut running, 20);
    assert_eq!(
        running.sent(),
        vec![
            WlCommand::Close(WindowKey(2)),
            WlCommand::Close(WindowKey(4))
        ]
    );
    assert_eq!(
        presence(&running.harness, "calc").as_deref(),
        Some("leaving")
    );
    wait(&mut running, 700);
    assert_eq!(panels(&running), 1, "the panel stays");
    assert_eq!(
        tiles(&running.harness),
        (names(&["term", "edit"]), Some("edit".to_owned())),
        "the tile went; the selection stays in place"
    );
}

#[test]
fn h_hides_the_selected_app_and_stays() {
    let mut running = shown();
    running
        .harness
        .chord(&[ShortcutKey::Alt], ShortcutKey::Char('h'));
    wait(&mut running, 20);
    assert_eq!(
        running.sent(),
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
    // The third cell: the first's rect, two cells (112) and gaps (8) along.
    let first = running
        .harness
        .rect(".ds-switcher-cell")
        .expect("the tiles are laid out"); // test-only
    let at = ds::Point {
        x: ds::Px(first.origin.x.0 + 2.0 * 120.0 + first.size.width.0 / 2.0),
        y: ds::Px(first.origin.y.0 + first.size.height.0 / 2.0),
    };
    running.harness.pointer_move(at);
    wait(&mut running, 20);
    assert_eq!(
        tiles(&running.harness).1.as_deref(),
        Some("edit"),
        "hover selects"
    );
    running.harness.click(at);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    assert_eq!(running.sent(), vec![WlCommand::Activate(WindowKey(3))]);
}

#[test]
fn made_up_apps_show_at_once_and_touch_no_window() {
    let mut running = running();
    wait(&mut running, 40);
    let apps = ["files", "mail", "notes"]
        .map(|a| AppId(a.to_owned()))
        .to_vec();
    running.ask(SwitcherAsk::Fake(apps));
    // Shown at the ask, and the key target's `focus_soon` lands in the same frame, every run:
    // no `FRAME_SLACK` allowance.
    wait(&mut running, 0);
    assert_eq!(
        tiles(&running.harness),
        (names(&["files", "mail", "notes"]), Some("mail".to_owned()))
    );
    assert!(
        running.harness.is_focused(".sill-switcher"),
        "focused at the ask"
    );
    running
        .harness
        .chord(&[ShortcutKey::Alt], ShortcutKey::Char('q'));
    running.harness.chord(&[ShortcutKey::Alt], ShortcutKey::Alt);
    wait(&mut running, 20);
    assert_eq!(panels(&running), 0);
    assert_eq!(running.sent(), vec![]);
}
