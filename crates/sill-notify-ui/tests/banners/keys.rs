//! The banners' keyboard: they appear without taking focus; `sill focus-latest` (the link asks,
//! naming the banners when they appeared last) gives the newest card the keyboard and pauses its
//! hold; Up and Down move along the stack, Enter presses the card, Tab reaches its buttons,
//! Enter presses a button too, Escape, Delete and Backspace dismiss the card and hand the focus on.

use ds_harness::{Driver, FocusState, Input, Query};
use std::time::Duration;

use ds::ShortcutKey;
use sill_model::notifications::{ActionKey, Activation, NotificationId, NotificationsCommand};
use sill_ui_kit::links::Transient;

use super::{Done, HOLD_MS, Running, commands, file, leaving, live, note, one, show, start, until};

const NEWEST: &str = "#toast-2";
const OLDER: &str = "#toast-1";

/// Two apps' banners (mail's newer), shown and entered, the keyboard asked for and given a few
/// frames to land.
fn two() -> Running {
    let mut running = start(file());
    show(
        &mut running,
        vec![
            live(note(2, "chat", "Newer", "b", &[("default", "Open")])),
            live(note(1, "mail", "Older", "a", &[("reply", "Reply")])),
        ],
    );
    assert_eq!(
        until(&mut running, Duration::from_millis(900), |r| super::cards(
            r
        ) == 2),
        Done::Yes,
        "both slide in"
    );
    running.harness.advance(Duration::from_millis(400));
    running
}

fn focused() -> Running {
    let mut running = two();
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(
        running.harness.focus_of(NEWEST),
        FocusState::Focused,
        "the newest card has the keyboard"
    );
    running
}

fn press(running: &mut Running, key: ShortcutKey) {
    running.harness.send(Input::key(key));
    running.harness.advance(Duration::from_millis(60));
}

fn dismissed(id: u32) -> NotificationsCommand {
    NotificationsCommand::Dismiss(NotificationId(id))
}

#[test]
fn they_appear_without_the_keyboard_and_keys_do_nothing_until_it_is_asked_for() {
    let mut running = two();
    assert!(
        running.harness.focus_of(NEWEST) == FocusState::Unfocused
            && running.harness.focus_of(OLDER) == FocusState::Unfocused
    );
    assert_ne!(
        running.harness.attr(".ds", "data-modality").as_deref(),
        Some("keyboard"),
        "no ring yet"
    );
    for key in [
        ShortcutKey::Enter,
        ShortcutKey::Delete,
        ShortcutKey::Escape,
        ShortcutKey::Down,
    ] {
        press(&mut running, key);
    }
    assert_eq!(commands(&mut running), Vec::new(), "no key reached a card");
    assert_eq!(leaving(&running), 0);
}

#[test]
fn asking_focuses_the_newest_card_with_its_ring() {
    let running = focused();
    assert_eq!(
        running.harness.attr(".ds", "data-modality").as_deref(),
        Some("keyboard"),
        "the ring shows"
    );
}

#[test]
fn only_the_holders_hold_pauses_and_it_resumes_when_the_keyboard_is_lost() {
    let timed_out = |id| vec![NotificationsCommand::BannerTimedOut(NotificationId(id))];
    let mut running = focused();
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS + 800));
    assert_eq!(
        commands(&mut running),
        timed_out(1),
        "the other card timed out, the held one did not"
    );
    running.link.keyboard_lost(Transient::Banner);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        commands(&mut running),
        Vec::new(),
        "the rest is not spent yet"
    );
    running.harness.advance(Duration::from_millis(3000));
    assert_eq!(
        commands(&mut running),
        timed_out(2),
        "the newest resumes and times out"
    );
}

#[test]
fn arrows_move_along_the_stack_and_stop_at_its_ends() {
    let mut running = focused();
    press(&mut running, ShortcutKey::Up);
    assert_eq!(
        running.harness.focus_of(NEWEST),
        FocusState::Focused,
        "nothing above the newest"
    );
    press(&mut running, ShortcutKey::Down);
    assert_eq!(running.harness.focus_of(OLDER), FocusState::Focused);
    press(&mut running, ShortcutKey::Down);
    assert_eq!(
        running.harness.focus_of(OLDER),
        FocusState::Focused,
        "nothing below the oldest"
    );
    press(&mut running, ShortcutKey::Up);
    assert_eq!(running.harness.focus_of(NEWEST), FocusState::Focused);
}

#[test]
fn enter_presses_the_focused_card() {
    let mut running = focused();
    press(&mut running, ShortcutKey::Enter);
    assert_eq!(
        commands(&mut running),
        vec![NotificationsCommand::InvokeAction {
            id: NotificationId(2),
            action: ActionKey::default_action(),
            token: Activation::NoToken,
        }]
    );
    assert_eq!(leaving(&running), 1);
}

#[test]
fn escape_delete_and_backspace_dismiss_and_the_focus_moves_on() {
    for key in [
        ShortcutKey::Escape,
        ShortcutKey::Delete,
        ShortcutKey::Backspace,
    ] {
        let mut running = focused();
        press(&mut running, key);
        assert_eq!(commands(&mut running), vec![dismissed(2)], "{key:?}");
        assert_eq!(
            running.harness.focus_of(OLDER),
            FocusState::Focused,
            "{key:?}: on to the next card"
        );
        press(&mut running, key);
        assert_eq!(
            commands(&mut running),
            vec![dismissed(1)],
            "{key:?}: the last one"
        );
        assert_eq!(leaving(&running), 2, "{key:?}");
    }
}

#[test]
fn tab_reaches_the_cards_buttons_and_the_card_keys_still_act_there() {
    let mut running = one(&[("reply", "Reply")]);
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(running.harness.focus_of("#toast-1"), FocusState::Focused);
    let mut reached = false;
    for _ in 0..4 {
        press(&mut running, ShortcutKey::Tab);
        reached |= running
            .harness
            .focus_of(".ds-notification-actions .ds-button")
            == FocusState::Focused;
    }
    assert!(reached, "Tab reaches the Reply button");
    press(&mut running, ShortcutKey::Escape);
    assert_eq!(
        commands(&mut running),
        vec![dismissed(1)],
        "Escape from a button"
    );
}

#[test]
fn a_second_ask_brings_the_keyboard_back_to_the_holder() {
    let mut running = focused();
    press(&mut running, ShortcutKey::Down);
    press(&mut running, ShortcutKey::Tab);
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(running.harness.focus_of(OLDER), FocusState::Focused);
}

#[test]
fn the_ask_goes_to_the_newer_of_the_thumbnail_and_the_banners() {
    // A thumbnail that appeared after the banners is asked, and the banners are left alone.
    let mut running = two();
    running.link.appeared(Transient::Thumbnail);
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(
        running.harness.focus_of(NEWEST),
        FocusState::Unfocused,
        "the thumbnail was the newer"
    );
    // A banner that appears after it takes the next ask.
    show(
        &mut running,
        vec![
            live(note(3, "cal", "Newest", "c", &[])),
            live(note(2, "chat", "Newer", "b", &[])),
            live(note(1, "mail", "Older", "a", &[])),
        ],
    );
    running.harness.advance(Duration::from_millis(400));
    running.link.ask_keyboard();
    running.harness.advance(Duration::from_millis(200));
    assert_eq!(
        running.harness.focus_of("#toast-3"),
        FocusState::Focused,
        "the newest banner is now the newer"
    );
}
