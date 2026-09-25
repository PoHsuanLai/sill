//! Table tests for `deck.rs` and `deck_cards.rs`.

use super::{Deck, DeckIn, DeckOut, DeckParams, EntryKey, Pose, Showing, advance, told};
use crate::surfaces::banners::machine::{Banner, BannerIn, Deadline, Leave};
use crate::surfaces::banners::stack::Card;
use crate::surfaces::notification_card::Expansion;
use sill_services::{AppKey, BannerLife, Cpx, NotificationId, NotificationsCommand, Stamp};
use sill_settings::{NotificationSwipe, NotificationsSettings};

fn p() -> DeckParams {
    DeckParams::new(&NotificationsSettings {
        swipe: NotificationSwipe::Dismiss,
        ..NotificationsSettings::default()
    })
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

/// One card from app `a`, risen and entered at 100.
fn shown() -> Vec<(u64, DeckIn)> {
    vec![
        (0, DeckIn::Cards(vec![card("a", &[1])])),
        (30, DeckIn::Risen(A)),
        (100, DeckIn::Card(A, BannerIn::Entered)),
    ]
}

fn with(mut base: Vec<(u64, DeckIn)>, more: Vec<(u64, DeckIn)>) -> Vec<(u64, DeckIn)> {
    base.extend(more);
    base
}

#[test]
fn a_card_rises_enters_holds_and_leaves_by_itself() {
    let (deck, out) = run(shown());
    assert_eq!(
        out,
        vec![DeckOut::Rise(A), DeckOut::Entrance(A)],
        "rise a frame later, then play the entrance"
    );
    assert_eq!(deck.drawn()[0].pose, Pose::Rest);
    assert_eq!(deck.next_wake(), Some(Stamp(5300)));
    assert_eq!(deck.showing(), Showing::Cards);
    let (deck, out) = run(with(shown(), vec![(5300, DeckIn::Tick)]));
    assert_eq!(
        out[2..],
        [
            DeckOut::Send(NotificationsCommand::BannerTimedOut(NotificationId(1))),
            DeckOut::Exit(A),
        ]
    );
    assert_eq!(deck.drawn()[0].pose, Pose::Offstage, "slides off the edge");
    let (deck, _) = run(with(
        shown(),
        vec![
            (5300, DeckIn::Tick),
            (5310, DeckIn::Cards(Vec::new())),
            (5600, DeckIn::Card(A, BannerIn::Left)),
        ],
    ));
    assert_eq!(deck.showing(), Showing::Nothing, "gone once the exit played");
}

#[test]
fn the_same_app_updates_its_card_and_holds_again() {
    let (deck, out) = run(with(
        shown(),
        vec![(3000, DeckIn::Cards(vec![card("a", &[2, 1])]))],
    ));
    assert_eq!(out.len(), 2, "no second card: {out:?}");
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
    let order: Vec<&str> = deck.drawn().iter().map(|e| e.card.app.0.as_str()).collect();
    assert_eq!(order, ["b", "a"], "newest on top");
}

#[test]
fn a_card_the_service_dropped_leaves_silently_and_a_leftover_is_not_news() {
    let (deck, out) = run(with(shown(), vec![(900, DeckIn::Cards(Vec::new()))]));
    assert_eq!(out[2..], [DeckOut::Exit(A)], "no Dismiss for the service's own drop");
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
        out[2..],
        [
            DeckOut::Send(NotificationsCommand::Dismiss(NotificationId(1))),
            DeckOut::Exit(A),
        ]
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
}

#[test]
fn hover_expands_and_a_card_under_the_pointer_is_hovered_once_shown() {
    let (deck, _) = run(with(
        shown(),
        vec![(4000, DeckIn::Card(A, BannerIn::HoverIn))],
    ));
    assert_eq!(deck.drawn()[0].expansion, Expansion::Expanded);
    assert_eq!(deck.next_wake(), None, "paused");
    let (deck, _) = run(with(
        shown(),
        vec![
            (4000, DeckIn::Card(A, BannerIn::HoverIn)),
            (7000, DeckIn::Tick),
            (7000, DeckIn::Card(A, BannerIn::HoverOut)),
        ],
    ));
    assert_eq!(deck.drawn()[0].expansion, Expansion::Collapsed);
    assert_eq!(deck.next_wake(), Some(Stamp(8500)));
    // Hovered while it was still entering: hovered as soon as it has entered.
    let (deck, _) = run(vec![
        (0, DeckIn::Cards(vec![card("a", &[1])])),
        (30, DeckIn::Risen(A)),
        (50, DeckIn::Card(A, BannerIn::HoverIn)),
        (100, DeckIn::Card(A, BannerIn::Entered)),
    ]);
    assert!(matches!(deck.drawn()[0].banner, Banner::Hovered { .. }));
}

#[test]
fn a_drag_follows_springs_back_or_flies_out() {
    let dragged = |dx: i32, v: i32| {
        run(with(
            shown(),
            vec![
                (500, DeckIn::Card(A, BannerIn::DragStart)),
                (
                    600,
                    DeckIn::Card(A, BannerIn::DragMove { dx: Cpx::px(dx) }),
                ),
            ],
        ))
        .0
        .drawn()[0]
            .pose
            .eq(&Pose::Follow(Cpx::px(v)))
    };
    assert!(dragged(40, 40), "1:1 to the right");
    assert!(dragged(-40, -10), "damped to the left");
    let released = |dx: i32| {
        run(with(
            shown(),
            vec![
                (500, DeckIn::Card(A, BannerIn::DragStart)),
                (
                    600,
                    DeckIn::Card(A, BannerIn::DragMove { dx: Cpx::px(dx) }),
                ),
                (
                    700,
                    DeckIn::Card(A, BannerIn::DragEnd { velocity_px_s: 100 }),
                ),
            ],
        ))
    };
    let (deck, _) = released(50);
    assert_eq!(deck.drawn()[0].pose, Pose::Rest, "springs back");
    let (deck, out) = released(90);
    assert_eq!(deck.drawn()[0].pose, Pose::Offstage, "flies out");
    assert_eq!(
        out[2..],
        [
            DeckOut::Send(NotificationsCommand::Dismiss(NotificationId(1))),
            DeckOut::Exit(A),
        ]
    );
    assert!(matches!(
        deck.drawn()[0].banner,
        Banner::Leaving {
            why: Leave::Dismissed,
            ..
        }
    ));
}

#[test]
fn an_acted_card_leaves_without_a_leave_command() {
    let (_, out) = run(with(
        shown(),
        vec![(900, DeckIn::Card(A, BannerIn::Clicked))],
    ));
    assert_eq!(out[2..], [DeckOut::Exit(A)]);
}

#[test]
fn a_swipe_keeps_the_notification_in_the_center_unless_set_to_dismiss() {
    use NotificationSwipe::{Dismiss, KeepInCenter};
    let end = BannerIn::DragEnd { velocity_px_s: 900 };
    let swipe = BannerIn::Swipe {
        dx: Cpx::px(100),
        velocity_px_s: 0,
    };
    let cases = [
        (Leave::Dismissed, end, KeepInCenter, Leave::Timeout),
        (Leave::Dismissed, swipe, KeepInCenter, Leave::Timeout),
        (Leave::Dismissed, end, Dismiss, Leave::Dismissed),
        (Leave::Dismissed, BannerIn::Close, KeepInCenter, Leave::Dismissed),
        (Leave::Timeout, BannerIn::Tick, KeepInCenter, Leave::Timeout),
        (Leave::Acted, BannerIn::Clicked, KeepInCenter, Leave::Acted),
    ];
    for (why, input, swipe, want) in cases {
        assert_eq!(told(why, input, swipe), want, "{why:?} {input:?} {swipe:?}");
    }
}
