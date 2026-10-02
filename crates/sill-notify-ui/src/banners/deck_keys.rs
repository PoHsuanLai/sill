//! The deck's keyboard, pure: which card holds it. The person asks for it (`sill focus-latest`),
//! it goes to the newest listed card and pauses that card's hold; arrows move it to the neighbour
//! card; the dismiss keys close the card that holds it and hand it to the next card, or give it
//! back when none is left; the compositor taking it away gives it back. A held card that leaves
//! any other way (a press, a swipe, the service dropping it) gives it back too, and no exit
//! leaves it taken.

use ds::base::time::stamp::Stamp;

use super::deck::{Deck, DeckOut, Entry, EntryKey, KeyFocus, Speak, feed};
use super::step::{BannerIn, BannerParams};

/// Which way along the stack, in age: the stack lists cards newest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighbour {
    Newer,
    Older,
}

/// What happened to the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeysIn {
    /// `sill focus-latest` chose the banners: the keyboard goes to the newest card (or comes
    /// back to the one that holds it).
    Asked,
    /// The compositor took the keyboard away.
    Lost,
    /// An arrow: the keyboard moves to the neighbour card; at the end of the stack it stays.
    Moved(Neighbour),
    /// Escape, Delete or Backspace: the card that holds it is dismissed.
    Dismissed,
}

/// `deck` after the keyboard's `input` at `now`.
pub(super) fn keys(
    deck: Deck,
    input: KeysIn,
    now: Stamp,
    p: &BannerParams,
) -> (Deck, Vec<DeckOut>) {
    let mut out = Vec::new();
    let listed: Vec<EntryKey> = deck.listed().iter().map(|e| e.key).collect();
    let held = deck
        .entries
        .iter()
        .find(|e| e.keys == KeyFocus::Held && !e.leaving())
        .map(|e| e.key);
    let next_to = |key: EntryKey, side: Neighbour| {
        let at = listed.iter().position(|k| *k == key)?;
        match side {
            Neighbour::Newer => at.checked_sub(1),
            Neighbour::Older => Some(at + 1),
        }
        .and_then(|n| listed.get(n).copied())
    };
    let give = |deck: Deck, from: Option<EntryKey>, to: EntryKey, out: &mut Vec<DeckOut>| {
        let deck = match from {
            Some(from) => update(deck, from, BannerIn::KeysOut, now, p, out),
            None => deck,
        };
        update(deck, to, BannerIn::KeysIn, now, p, out)
    };
    let deck = match (input, held) {
        (KeysIn::Asked, Some(key)) => {
            out.push(DeckOut::TakeKeyboard(key));
            deck
        }
        (KeysIn::Asked, None) => match listed.first().copied() {
            Some(key) => {
                out.push(DeckOut::TakeKeyboard(key));
                give(deck, None, key, &mut out)
            }
            None => deck,
        },
        (KeysIn::Lost, Some(key)) => {
            out.push(DeckOut::ReleaseKeyboard);
            update(deck, key, BannerIn::KeysOut, now, p, &mut out)
        }
        (KeysIn::Moved(side), Some(key)) => match next_to(key, side) {
            Some(next) => {
                out.push(DeckOut::FocusCard(next));
                give(deck, Some(key), next, &mut out)
            }
            None => deck,
        },
        (KeysIn::Dismissed, Some(key)) => {
            let next = next_to(key, Neighbour::Older).or_else(|| next_to(key, Neighbour::Newer));
            let deck = update(deck, key, BannerIn::Close, now, p, &mut out);
            match next {
                Some(next) => {
                    out.push(DeckOut::FocusCard(next));
                    give(deck, Some(key), next, &mut out)
                }
                None => deck,
            }
        }
        (_, None) => deck,
    };
    release_leavers(deck, out)
}

/// `deck` with `input` fed to the card `key`.
fn update(
    deck: Deck,
    key: EntryKey,
    input: BannerIn,
    now: Stamp,
    p: &BannerParams,
    out: &mut Vec<DeckOut>,
) -> Deck {
    let entries = deck
        .entries
        .into_iter()
        .map(|e| match e.key == key {
            true => feed(e, input, now, p, out, Speak::Yes),
            false => e,
        })
        .collect();
    Deck { entries, ..deck }
}

/// A card that has started to leave no longer holds the keyboard; if that leaves nobody holding
/// it, it goes back.
pub(super) fn release_leavers(deck: Deck, mut out: Vec<DeckOut>) -> (Deck, Vec<DeckOut>) {
    let left = |e: &Entry| e.leaving() && e.keys == KeyFocus::Held;
    if !deck.entries.iter().any(left) {
        return (deck, out);
    }
    let entries: Vec<Entry> = deck
        .entries
        .into_iter()
        .map(|e| match left(&e) {
            true => Entry {
                keys: KeyFocus::Away,
                ..e
            },
            false => e,
        })
        .collect();
    if !entries.iter().any(|e| e.keys == KeyFocus::Held) {
        out.push(DeckOut::ReleaseKeyboard);
    }
    (Deck { entries, ..deck }, out)
}
