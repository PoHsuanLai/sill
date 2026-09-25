//! The center's keys on the real surface (design/20 §1.6): the column takes the keyboard as it
//! opens; Down and Up move the ring over the drawn rows, Return opens a stack or invokes a lone
//! card's `default` and closes, Delete dismisses a lone card or clears a stack, Escape closes.

mod center_support;

use center_support::{opened, ringed, two_apps};
use ds::Key;
use sill_services::{ActionKey, Activation, AppKey, Clear, NotificationId, NotificationsCommand};
use sill_surfaces::surfaces::notification_center::CenterShown;

#[test]
fn the_column_holds_the_keyboard_and_the_arrows_move_the_ring() {
    let mut running = opened(two_apps());
    assert!(
        running.harness.is_focused(".sill-nc-column"),
        "focused as it opens"
    );
    assert_eq!(ringed(&running.harness), None);
    let steps = [
        (Key::Down, Some("3")),
        (Key::Down, Some("fold")),
        (Key::Down, Some("1")),
        (Key::Down, Some("1")),
        (Key::Up, Some("fold")),
    ];
    for (key, want) in steps {
        running.harness.key(key);
        running.advance(20);
        assert_eq!(ringed(&running.harness).as_deref(), want, "{key:?}");
    }
}

#[test]
fn return_opens_a_stack_then_invokes_a_lone_card_and_closes() {
    let mut running = opened(two_apps());
    running.harness.key(Key::Down);
    running.harness.key(Key::Enter);
    running.advance(40);
    assert_eq!(running.harness.count(".sill-nc-row"), 4, "the stack opened");
    assert!(running.commands().is_empty());
    running.harness.key(Key::Enter);
    running.advance(40);
    assert_eq!(
        running.commands(),
        vec![NotificationsCommand::InvokeAction {
            id: NotificationId(5),
            action: ActionKey::default_action(),
            token: Activation::NoToken,
        }]
    );
    assert_eq!(running.link.now(), CenterShown::Closed);
}

#[test]
fn delete_clears_a_stack_and_dismisses_a_lone_card() {
    let mut running = opened(two_apps());
    running.harness.key(Key::Down);
    running.harness.key(Key::Delete);
    running.advance(20);
    // The service has not answered, so the rows are still the stack, its fold line and Chat's
    // card; the ring stayed on the stack. Backspace is the Mac keyboard's Delete.
    running.harness.key(Key::Down);
    running.harness.key(Key::Down);
    running.harness.key(Key::Backspace);
    running.advance(20);
    assert_eq!(
        running.commands(),
        vec![
            NotificationsCommand::ClearHistory(Clear::App(AppKey("mail".to_owned()))),
            NotificationsCommand::Dismiss(NotificationId(4)),
        ]
    );
}

#[test]
fn escape_closes() {
    let mut running = opened(two_apps());
    running.harness.key(Key::Escape);
    running.advance(20);
    assert_eq!(running.link.now(), CenterShown::Closed);
    assert!(running.commands().is_empty());
}
