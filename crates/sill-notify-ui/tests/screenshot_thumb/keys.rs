//! The thumbnail's keyboard: it appears without taking focus; `sill focus-latest` (the banners'
//! link asks) gives it the keyboard, pauses the hold, and then Enter and Space open it, Delete
//! and Backspace trash it, Escape dismisses it and Tab moves between its buttons.

use ds_harness::{Driver, FocusState, Input, Query};
use std::time::Duration;

use ds::ShortcutKey;
use sill_model::screenshots::ShotLeave;
use sill_settings::ShellFile;

use super::{Done, HOLD_MS, Running, commands, file, leave, leaving, one_up, start, until};

const PICTURE: &str = ".ds-shot-picture";
const ACTION: &str = ".ds-shot-actions .ds-button";

/// The card up, then the keyboard asked for and given a few frames to land.
fn focused(running: &mut Running) {
    one_up(running);
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(
        running.harness.focus_of(PICTURE),
        FocusState::Focused,
        "the picture has the keyboard"
    );
}

fn modality(running: &Running) -> Option<String> {
    running.harness.attr(".ds", "data-modality")
}

#[test]
fn it_appears_without_the_keyboard_and_keys_do_nothing_until_it_is_asked_for() {
    let mut running = start(file());
    one_up(&mut running);
    assert_eq!(
        running.harness.focus_of(PICTURE),
        FocusState::Unfocused,
        "no focus on appear"
    );
    assert_ne!(
        modality(&running).as_deref(),
        Some("keyboard"),
        "no ring yet"
    );
    for key in [ShortcutKey::Enter, ShortcutKey::Delete, ShortcutKey::Escape] {
        running.harness.send(Input::key(key));
        running.harness.advance(Duration::from_millis(40));
    }
    assert_eq!(
        commands(&mut running),
        Vec::new(),
        "no key reached the card"
    );
    assert_eq!(leaving(&running), 0);
}

#[test]
fn asking_for_the_keyboard_focuses_the_picture_with_its_ring() {
    let mut running = start(file());
    focused(&mut running);
    assert_eq!(
        modality(&running).as_deref(),
        Some("keyboard"),
        "the ring shows"
    );
}

#[test]
fn the_hold_pauses_while_the_keyboard_is_held_and_resumes_when_it_is_lost() {
    let mut running = start(file());
    focused(&mut running);
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS + 800));
    assert_eq!(leaving(&running), 0, "paused past the hold");
    assert_eq!(commands(&mut running), Vec::new());
    running
        .link
        .keyboard_lost(sill_ui_kit::links::Transient::Thumbnail);
    let rest = u64::from(ShellFile::default().notifications.hover_min_remaining_ms.0);
    running.harness.advance(Duration::from_millis(rest - 300));
    assert_eq!(leaving(&running), 0, "the rest after the keyboard goes");
    assert_eq!(
        until(&mut running, Duration::from_millis(900), |r| leaving(r)
            == 1),
        Done::Yes
    );
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
}

#[test]
fn enter_and_space_open_it() {
    for key in [ShortcutKey::Enter, ShortcutKey::Space] {
        let mut running = start(file());
        focused(&mut running);
        running.harness.send(Input::key(key));
        running.harness.advance(Duration::from_millis(40));
        assert_eq!(
            commands(&mut running),
            leave(1, ShotLeave::Opened),
            "{key:?}"
        );
        assert_eq!(leaving(&running), 1, "{key:?}");
    }
}

#[test]
fn delete_and_backspace_trash_it() {
    for key in [ShortcutKey::Delete, ShortcutKey::Backspace] {
        let mut running = start(file());
        focused(&mut running);
        running.harness.send(Input::key(key));
        running.harness.advance(Duration::from_millis(40));
        assert_eq!(
            commands(&mut running),
            leave(1, ShotLeave::Trashed),
            "{key:?}"
        );
        assert_eq!(leaving(&running), 1, "{key:?}");
    }
}

#[test]
fn escape_dismisses_it_and_keeps_the_file() {
    let mut running = start(file());
    focused(&mut running);
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(commands(&mut running), leave(1, ShotLeave::Kept));
    assert_eq!(leaving(&running), 1, "it slides out");
}

#[test]
fn tab_moves_between_the_picture_and_its_button_and_the_card_keys_still_act_there() {
    let mut running = start(file());
    focused(&mut running);
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.focus_of(ACTION),
        FocusState::Focused,
        "Tab reaches the Delete button"
    );
    assert_eq!(running.harness.focus_of(PICTURE), FocusState::Unfocused);
    running
        .harness
        .send(Input::chord(&[ShortcutKey::Shift], ShortcutKey::Tab));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.focus_of(PICTURE),
        FocusState::Focused,
        "Shift+Tab goes back"
    );
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        commands(&mut running),
        leave(1, ShotLeave::Kept),
        "Escape from the button"
    );
}

#[test]
fn a_second_ask_brings_the_keyboard_back_to_the_picture() {
    let mut running = start(file());
    focused(&mut running);
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.advance(Duration::from_millis(40));
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(running.harness.focus_of(PICTURE), FocusState::Focused);
}

#[test]
fn the_ask_goes_to_the_thumbnail_only_when_it_appeared_after_the_banners() {
    use sill_ui_kit::links::Transient;
    let mut older_banner = start(file());
    older_banner.link.appeared(Transient::Banner);
    one_up(&mut older_banner);
    older_banner.link.ask_keyboard();
    older_banner.harness.advance(Duration::from_millis(200));
    assert_eq!(
        older_banner.harness.focus_of(PICTURE),
        FocusState::Focused,
        "the thumbnail is newer"
    );

    let mut newer_banner = start(file());
    one_up(&mut newer_banner);
    newer_banner.link.appeared(Transient::Banner);
    newer_banner.link.ask_keyboard();
    newer_banner.harness.advance(Duration::from_millis(200));
    assert_eq!(
        newer_banner.harness.focus_of(PICTURE),
        FocusState::Unfocused,
        "a banner is newer"
    );
}
