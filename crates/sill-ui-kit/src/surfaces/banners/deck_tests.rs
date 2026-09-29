//! Table tests for `deck.rs` and `deck_cards.rs`.

use super::{Deck, DeckIn, DeckOut, EntryKey, Showing, advance};
use crate::surfaces::banners::machine::{Banner, BannerIn, BannerParams, Deadline, Leave};
use crate::surfaces::banners::stack::Card;
use sill_model::notifications::{AppKey, BannerLife, NotificationId, NotificationsCommand};
use sill_model::units::Stamp;
use sill_settings::NotificationsSettings;

fn p() -> BannerParams {
    BannerParams::new(&NotificationsSettings::default())
}

fn card(app: &str, ids: &[u32]) -> Card {
    Card {
        app: AppKey(app.to_owned()),
        grouped: ids.iter().map(|&id| NotificationId(id)).collect(),
        life: BannerLife::Expires,
    }
}

const A: EntryKey = EntryKey(0);
const B: EntryKey = EntryKey(1);

/// Run `inputs` (at their stamps) from an empty deck; every effect in order.
fn run(inputs: Vec<(u64, DeckIn)>) -> (Deck, Vec<DeckOut>) {
    let params = p();
    inputs.into_iter().fold(
        (Deck::default(), Vec::new()),
        |(deck, mut all), (ms, input)| {
            let (deck, out) = advance(deck, input, Stamp(ms), &params);
            all.extend(out);
            (deck, all)
        },
    )
}

/// One card from app `a`, risen (its hold starting) at 100.
fn shown() -> Vec<(u64, DeckIn)> {
    vec![
        (0, DeckIn::Cards(vec![card("a", &[1])])),
        (100, DeckIn::Risen(A)),
    ]
}

fn with(mut base: Vec<(u64, DeckIn)>, more: Vec<(u64, DeckIn)>) -> Vec<(u64, DeckIn)> {
    base.extend(more);
    base
}

fn listed(deck: &Deck) -> Vec<EntryKey> {
    deck.listed().iter().map(|e| e.key).collect()
}

#[test]
fn a_card_rises_holds_and_leaves_by_itself() {
    let (deck, out) = run(shown());
    assert_eq!(out, vec![DeckOut::Rise(A)], "rise a frame later");
    assert_eq!(listed(&deck), [A]);
    assert_eq!(deck.next_wake(), Some(Stamp(5300)));
    assert_eq!(deck.showing(), Showing::Cards);
    let (deck, out) = run(with(shown(), vec![(5300, DeckIn::Tick)]));
    assert_eq!(
        out[1..],
        [DeckOut::Send(NotificationsCommand::BannerTimedOut(
            NotificationId(1)
        ))]
    );
    assert_eq!(
        listed(&deck),
        [],
        "no longer listed: the stack plays it out"
    );
    assert_eq!(deck.drawn().len(), 1, "but still drawn");
    assert_eq!(deck.showing(), Showing::Cards);
    let (deck, _) = run(with(
        shown(),
        vec![
            (5300, DeckIn::Tick),
            (5310, DeckIn::Cards(Vec::new())),
            (5600, DeckIn::Card(A, BannerIn::Left)),
        ],
    ));
    assert_eq!(
        deck.showing(),
        Showing::Nothing,
        "gone once the stack says its exit played"
    );
}

#[test]
fn the_same_app_updates_its_card_and_holds_again() {
    let (deck, out) = run(with(
        shown(),
        vec![(3000, DeckIn::Cards(vec![card("a", &[2, 1])]))],
    ));
    assert_eq!(out.len(), 1, "no second card: {out:?}");
    let drawn = deck.drawn();
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].card.count(), 2);
    assert_eq!(
        drawn[0].banner,
        Banner::Shown {
            deadline: Deadline::At(Stamp(8200))
        },
        "the hold starts again for the newer one"
    );
    let (deck, _) = run(with(
        shown(),
        vec![(3000, DeckIn::Cards(vec![card("a", &[1]), card("b", &[2])]))],
    ));
    let order: Vec<&str> = deck
        .listed()
        .iter()
        .map(|e| e.card.app.0.as_str())
        .collect();
    assert_eq!(order, ["b", "a"], "newest first");
}

#[test]
fn a_card_the_service_dropped_leaves_silently_and_a_leftover_is_not_news() {
    let (deck, out) = run(with(shown(), vec![(900, DeckIn::Cards(Vec::new()))]));
    assert_eq!(out[1..], [], "no Dismiss for the service's own drop");
    assert!(deck.drawn()[0].leaving());
    // Closed, and before the service caught up the stack still has it: no second card.
    let (deck, out) = run(with(
        shown(),
        vec![
            (900, DeckIn::Card(A, BannerIn::Close)),
            (910, DeckIn::Cards(vec![card("a", &[1])])),
        ],
    ));
    assert_eq!(
        out[1..],
        [DeckOut::Send(NotificationsCommand::Dismiss(
            NotificationId(1)
        ))]
    );
    assert_eq!(deck.drawn().len(), 1);
    // A newer one from the same app while the old card leaves is a new card.
    let (deck, out) = run(with(
        shown(),
        vec![
            (900, DeckIn::Card(A, BannerIn::Close)),
            (910, DeckIn::Cards(vec![card("a", &[3, 1])])),
        ],
    ));
    assert_eq!(out.last(), Some(&DeckOut::Rise(B)));
    assert_eq!(deck.drawn().len(), 2);
    assert_eq!(listed(&deck), [B]);
}

#[test]
fn hover_pauses_and_a_card_under_the_pointer_is_hovered_once_shown() {
    let (deck, _) = run(with(
        shown(),
        vec![(4000, DeckIn::Card(A, BannerIn::HoverIn))],
    ));
    assert_eq!(deck.next_wake(), None, "paused");
    let (deck, _) = run(with(
        shown(),
        vec![
            (4000, DeckIn::Card(A, BannerIn::HoverIn)),
            (7000, DeckIn::Tick),
            (7000, DeckIn::Card(A, BannerIn::HoverOut)),
        ],
    ));
    assert_eq!(deck.next_wake(), Some(Stamp(8500)));
    // Under the pointer before it rose: hovered as soon as it rises.
    let (deck, _) = run(vec![
        (0, DeckIn::Cards(vec![card("a", &[1])])),
        (50, DeckIn::Card(A, BannerIn::HoverIn)),
        (100, DeckIn::Risen(A)),
    ]);
    assert!(matches!(deck.drawn()[0].banner, Banner::Hovered { .. }));
}

#[test]
fn leaving_tells_the_service_why() {
    use NotificationsCommand::{Dismiss, SwipeAway};
    let cases = [
        (
            BannerIn::Swiped,
            vec![SwipeAway(NotificationId(1))],
            Leave::Swiped,
        ),
        (
            BannerIn::Close,
            vec![Dismiss(NotificationId(1))],
            Leave::Dismissed,
        ),
        (BannerIn::Clicked, vec![], Leave::Acted),
    ];
    for (input, sent, why) in cases {
        let (deck, out) = run(with(shown(), vec![(900, DeckIn::Card(A, input))]));
        let want: Vec<DeckOut> = sent.into_iter().map(DeckOut::Send).collect();
        assert_eq!(out[1..], want, "{input:?}");
        assert!(
            matches!(deck.drawn()[0].banner, Banner::Leaving { why: w, .. } if w == why),
            "{input:?}"
        );
        assert_eq!(listed(&deck), [], "{input:?}");
    }
}
