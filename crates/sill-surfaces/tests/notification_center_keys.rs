//! The center's keys on the real surface (design/20 §1.6): the column takes the keyboard as it
//! opens; Down and Up move the ring over the drawn rows (each group's header, then its cards),
//! Return opens a group from its header or its stack or invokes a lone card's `default` and
//! closes, Delete clears a group from its header or its stack or dismisses a lone card, Escape
//! closes.

mod center_support;

use center_support::{opened, ringed, two_apps};
use ds::ShortcutKey;
use sill_model::notifications::{
    ActionKey, Activation, AppKey, Clear, NotificationId, NotificationsCommand,
};
use sill_surfaces::links::CenterShown;

#[test]
fn the_column_holds_the_keyboard_and_the_arrows_move_the_ring() {
    let mut running = opened(two_apps());
    assert!(
        running.harness.is_focused(".sill-nc-column"),
        "focused as it opens"
    );
    assert_eq!(ringed(&running.harness), None);
    let steps = [
        (ShortcutKey::Down, Some("head")),
        (ShortcutKey::Down, Some("3")),
        (ShortcutKey::Down, Some("head")),
        (ShortcutKey::Down, Some("1")),
        (ShortcutKey::Down, Some("1")),
        (ShortcutKey::Up, Some("head")),
    ];
    for (key, want) in steps {
        running.harness.key(key);
        running.advance(20);
        assert_eq!(ringed(&running.harness).as_deref(), want, "{key:?}");
    }
}

#[test]
fn return_opens_a_group_then_invokes_a_lone_card_and_closes() {
    let mut running = opened(two_apps());
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Enter);
    running.advance(40);
    assert_eq!(running.harness.count(".sill-nc-row"), 4, "Mail opened");
    assert!(running.commands().is_empty());
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Enter);
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
fn return_on_a_stack_opens_it() {
    let mut running = opened(two_apps());
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Enter);
    running.advance(40);
    assert_eq!(running.harness.count(".sill-nc-row"), 4, "the stack opened");
    assert!(running.commands().is_empty());
}

#[test]
fn delete_clears_a_group_and_dismisses_a_lone_card() {
    let mut running = opened(two_apps());
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Delete);
    running.advance(20);
    // The service has not answered, so the rows are still Mail's header and stack and Chat's
    // header and card; the ring stayed on Mail's header. Backspace is the Mac keyboard's Delete.
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Delete);
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Down);
    running.harness.key(ShortcutKey::Backspace);
    running.advance(20);
    let mail = || NotificationsCommand::ClearHistory(Clear::App(AppKey("mail".to_owned())));
    assert_eq!(
        running.commands(),
        vec![
            mail(),
            mail(),
            NotificationsCommand::Dismiss(NotificationId(4)),
        ]
    );
}

#[test]
fn escape_closes() {
    let mut running = opened(two_apps());
    running.harness.key(ShortcutKey::Escape);
    running.advance(20);
    assert_eq!(running.link.now(), CenterShown::Closed);
    assert!(running.commands().is_empty());
}
