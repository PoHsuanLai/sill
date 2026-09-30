//! The deck following the stack: a card of an app not drawn arrives; a drawn app's card takes
//! the new content, and a newer notification starts its hold again; a drawn card the stack no
//! longer has leaves without a word to the service. A stack card whose newest notification is
//! one a leaving card already stood for is the service not having caught up yet, not news.

use sill_model::units::Stamp;

use super::deck::{Deck, DeckOut, Entry, KeyFocus, Speak, arrive, feed};
use super::stack::Card;
use super::step::{Banner, BannerIn, BannerParams, Persistence, step};

/// `deck` following `cards` at `now`.
pub(super) fn follow(
    deck: Deck,
    cards: Vec<Card>,
    now: Stamp,
    p: &BannerParams,
) -> (Deck, Vec<DeckOut>) {
    let mut out = Vec::new();
    let mut deck = deck;
    let (staying, gone): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut deck.entries)
        .into_iter()
        .partition(|e| e.leaving() || cards.iter().any(|c| c.app == e.card.app));
    let mut entries: Vec<Entry> = staying
        .into_iter()
        .map(|e| match cards.iter().find(|c| c.app == e.card.app) {
            Some(card) if !e.leaving() => refresh(e, card.clone(), now, p),
            _ => e,
        })
        .collect();
    entries.extend(
        gone.into_iter()
            .map(|e| feed(e, BannerIn::Close, now, p, &mut out, Speak::No)),
    );
    deck.entries = entries;
    for card in cards {
        let drawn = deck.entries.iter().any(|e| {
            (!e.leaving() && e.card.app == card.app)
                || (e.leaving() && card.newest().is_some_and(|id| e.card.grouped.contains(&id)))
        });
        if !drawn {
            arrive(&mut deck, card, now, &mut out);
        }
    }
    (deck, out)
}

/// A drawn card with `card`'s content; a newer notification starts a shown card's hold again.
fn refresh(e: Entry, card: Card, now: Stamp, p: &BannerParams) -> Entry {
    let newer = card.newest() != e.card.newest();
    let banner = match (newer, e.banner) {
        (true, Banner::Shown { .. }) => {
            let entering = Banner::Entering { since: now };
            step(
                entering,
                BannerIn::Entered,
                now,
                Persistence::of(card.life),
                p,
            )
            .0
        }
        (_, banner) => banner,
    };
    // A card the keyboard is on keeps its hold paused through a newer notification.
    let banner = match (e.keys, banner) {
        (KeyFocus::Held, Banner::Shown { .. }) => {
            step(
                banner,
                BannerIn::HoverIn,
                now,
                Persistence::of(card.life),
                p,
            )
            .0
        }
        (_, banner) => banner,
    };
    Entry { card, banner, ..e }
}
