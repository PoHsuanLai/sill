//! What a card's input tells the service: the one mapping from the banner machine's decisions
//! (`step::Leave`) and a card press (`notification_card::CardPress`) to
//! `NotificationsCommand`s. A card stands for every banner grouped under it, so leaving is said
//! for each; an action is the newest one's.

use sill_model::notifications::{ActionKey, Activation, NotificationsCommand};

use super::stack::Card;
use super::step::Leave;

/// What `card` leaving for `why` sends: its hold ran out (`BannerTimedOut`), it was closed
/// (`Dismiss`: the close button) or swiped away (`SwipeAway`: the service keeps it in the center
/// or dismisses it, as `notifications.swipe` says), once per grouped banner, newest first. Leaving because it was acted on sends
/// nothing here: [`action_command`] already said it when the press happened.
pub fn leave_commands(card: &Card, why: Leave) -> Vec<NotificationsCommand> {
    let command = match why {
        Leave::Timeout => NotificationsCommand::BannerTimedOut,
        Leave::Dismissed => NotificationsCommand::Dismiss,
        Leave::Swiped => NotificationsCommand::SwipeAway,
        Leave::Acted => return Vec::new(),
    };
    card.grouped.iter().copied().map(command).collect()
}

/// A press on `card`'s body (`ActionKey::default_action()`) or on one of its buttons: the
/// newest notification's action, with the surface's activation token when it got one.
/// `None` for a card with no notification (it never has none).
pub fn action_command(
    card: &Card,
    action: ActionKey,
    token: Activation,
) -> Option<NotificationsCommand> {
    card.newest()
        .map(|id| NotificationsCommand::InvokeAction { id, action, token })
}

#[cfg(test)]
mod tests {
    use super::{action_command, leave_commands};
    use crate::banners::stack::Card;
    use crate::banners::step::Leave;
    use sill_model::notifications::{
        ActionKey, Activation, ActivationToken, AppKey, BannerLife, NotificationId,
        NotificationsCommand,
    };

    fn card() -> Card {
        Card {
            app: AppKey("a".to_owned()),
            grouped: vec![NotificationId(3), NotificationId(1)],
            life: BannerLife::Expires,
        }
    }

    #[test]
    fn leaving_speaks_for_every_grouped_banner() {
        use NotificationsCommand::{BannerTimedOut, Dismiss, SwipeAway};
        let cases = [
            (
                Leave::Timeout,
                vec![
                    BannerTimedOut(NotificationId(3)),
                    BannerTimedOut(NotificationId(1)),
                ],
            ),
            (
                Leave::Dismissed,
                vec![Dismiss(NotificationId(3)), Dismiss(NotificationId(1))],
            ),
            (
                Leave::Swiped,
                vec![SwipeAway(NotificationId(3)), SwipeAway(NotificationId(1))],
            ),
            (Leave::Acted, vec![]),
        ];
        for (why, want) in cases {
            assert_eq!(leave_commands(&card(), why), want, "{why:?}");
        }
    }

    #[test]
    fn an_action_is_the_newest_ones() {
        let token = Activation::Token(ActivationToken("t".to_owned()));
        assert_eq!(
            action_command(&card(), ActionKey::default_action(), token.clone()),
            Some(NotificationsCommand::InvokeAction {
                id: NotificationId(3),
                action: ActionKey::default_action(),
                token,
            })
        );
        let empty = Card {
            grouped: Vec::new(),
            ..card()
        };
        assert_eq!(
            action_command(&empty, ActionKey::default_action(), Activation::NoToken),
            None
        );
    }
}
