//! Which cards the stack shows (design/13 §13.3.6 "Stack", "Grouping by app", acceptance 11):
//! the live banners grouped by app, one card per app showing its newest notification with a
//! count chip, cards ordered by their newest notification, newest first, at most
//! `notifications.stack_max` of them; the rest wait until a card leaves.

use sill_model::notifications::{AppKey, BannerLife, LiveBanner, NotificationId};

/// One card: an app's live banners, drawn as its newest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub app: AppKey,
    /// Every live banner the card stands for, newest first (never empty): the first is drawn,
    /// and all of them leave with the card.
    pub grouped: Vec<NotificationId>,
    /// `Persists` when any grouped banner persists: a later banner from the same app never makes
    /// an alert leave by itself.
    pub life: BannerLife,
}

impl Card {
    /// The notification the card draws.
    pub fn newest(&self) -> Option<NotificationId> {
        self.grouped.first().copied()
    }

    /// The count chip's number (1: no chip).
    pub fn count(&self) -> usize {
        self.grouped.len()
    }
}

/// The cards `banners` (newest first, as the service publishes them) make, at most `max`.
pub fn stack(banners: &[LiveBanner], max: usize) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::new();
    for banner in banners {
        let app = banner.note.app.key();
        match cards.iter_mut().find(|card| card.app == app) {
            Some(card) => {
                card.grouped.push(banner.note.id);
                card.life = longer(card.life, banner.life);
            }
            None => cards.push(Card {
                app,
                grouped: vec![banner.note.id],
                life: banner.life,
            }),
        }
    }
    cards.truncate(max);
    cards
}

/// The life of a card holding banners of lives `a` and `b`: an alert among them keeps it.
fn longer(a: BannerLife, b: BannerLife) -> BannerLife {
    match (a, b) {
        (BannerLife::Expires, BannerLife::Expires) => BannerLife::Expires,
        _ => BannerLife::Persists,
    }
}

#[cfg(test)]
mod tests {
    use super::{Card, stack};
    use sill_model::notifications::{
        AppKey, AppRef, BannerLife, Body, Expiry, Hints, IconRef, LiveBanner, Notification,
        NotificationId, UnixMs,
    };

    fn live(id: u32, app: &str, life: BannerLife) -> LiveBanner {
        LiveBanner {
            note: Notification {
                id: NotificationId(id),
                arrived: UnixMs(i64::from(id)),
                app: AppRef {
                    name: app.to_owned(),
                    icon: IconRef::None,
                    desktop_entry: None,
                },
                summary: String::new(),
                body: Body::default(),
                actions: Vec::new(),
                hints: Hints::default(),
                expiry: Expiry::Default,
            },
            life,
        }
    }

    fn card(app: &str, ids: &[u32], life: BannerLife) -> Card {
        Card {
            app: AppKey(app.to_owned()),
            grouped: ids.iter().map(|&id| NotificationId(id)).collect(),
            life,
        }
    }

    #[test]
    fn banners_group_by_app_newest_first_up_to_the_cap() {
        use BannerLife::{Expires, Persists};
        let e = |id, app| live(id, app, Expires);
        let cases: Vec<(&str, Vec<LiveBanner>, usize, Vec<Card>)> = vec![
            ("none", vec![], 3, vec![]),
            (
                "one app three times",
                vec![e(3, "a"), e(2, "a"), e(1, "a")],
                3,
                vec![card("a", &[3, 2, 1], Expires)],
            ),
            (
                "five apps",
                vec![e(5, "e"), e(4, "d"), e(3, "c"), e(2, "b"), e(1, "a")],
                3,
                vec![
                    card("e", &[5], Expires),
                    card("d", &[4], Expires),
                    card("c", &[3], Expires),
                ],
            ),
            (
                "interleaved",
                vec![e(3, "a"), e(2, "b"), e(1, "a")],
                3,
                vec![card("a", &[3, 1], Expires), card("b", &[2], Expires)],
            ),
            (
                "an alert keeps its card",
                vec![e(2, "a"), live(1, "a", Persists)],
                3,
                vec![card("a", &[2, 1], Persists)],
            ),
            (
                "cap of one",
                vec![e(2, "b"), e(1, "a")],
                1,
                vec![card("b", &[2], Expires)],
            ),
        ];
        for (name, banners, max, want) in cases {
            assert_eq!(stack(&banners, max), want, "{name}");
        }
    }

    #[test]
    fn a_card_draws_its_newest_and_counts_the_rest() {
        let c = card("a", &[3, 1], BannerLife::Expires);
        assert_eq!(c.newest(), Some(NotificationId(3)));
        assert_eq!(c.count(), 2);
    }
}
