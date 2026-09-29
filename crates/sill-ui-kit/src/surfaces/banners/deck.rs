//! The column's cards over time, pure: which cards the stack lists (the [`stack`](super::stack)'s)
//! and which are still drawn while quire's `BannerStack` plays them out, one banner
//! [`step`](super::step) per card, and what the column must do next (start a hold, tell the
//! service). The view (`io.rs`) owns the timers; the motion is quire's. Everything it decides
//! with is here, tested by table (`deck_tests.rs`).
//!
//! A card keeps its entry while its app's banners keep coming: a newer notification replaces
//! the content, bumps the count and starts the hold again (design/13 §13.3.6 "Grouping by app").
//! A card the service dropped by itself (`CloseNotification`, a replacement elsewhere, Do Not
//! Disturb's Dismiss All) leaves without telling the service anything. A leaving card is no
//! longer listed; it stays in the deck until the stack says its exit has played
//! (`BannerIn::Left`, from `on_hidden`).

use sill_model::notifications::NotificationsCommand;
use sill_model::units::Stamp;

use super::commands::leave_commands;
use super::stack::Card;
use super::step::{Banner, BannerIn, BannerOut, BannerParams, Deadline, Persistence, step};

/// One drawn card's identity for its life on screen (a new one per arrival): quire's
/// `BannerKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryKey(pub u32);

/// Whether the pointer is over a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pointer {
    Over,
    #[default]
    Away,
}

/// One drawn card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: EntryKey,
    pub card: Card,
    pub banner: Banner,
    pub pointer: Pointer,
}

impl Entry {
    /// Whether it has started to leave (or has gone).
    pub fn leaving(&self) -> bool {
        matches!(self.banner, Banner::Leaving { .. } | Banner::Gone)
    }
}

/// Every drawn card, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deck {
    pub entries: Vec<Entry>,
    next: u32,
}

/// What reaches the deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckIn {
    /// The stack the service's banners make now (this output's; none elsewhere).
    Cards(Vec<Card>),
    /// A frame has passed since the card mounted: its hold starts.
    Risen(EntryKey),
    /// Something happened to one card.
    Card(EntryKey, BannerIn),
    /// A hold may have run out.
    Tick,
}

/// What the column must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckOut {
    /// Answer with [`DeckIn::Risen`] a frame from now.
    Rise(EntryKey),
    /// Tell the service.
    Send(NotificationsCommand),
}

impl Deck {
    /// The cards still drawn, leaving ones included, newest first: the regions' cards.
    pub fn drawn(&self) -> Vec<&Entry> {
        let mut drawn: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| e.banner != Banner::Gone)
            .collect();
        drawn.sort_by_key(|e| std::cmp::Reverse(e.card.newest()));
        drawn
    }

    /// The cards the stack lists, newest first: every drawn one but those leaving.
    pub fn listed(&self) -> Vec<&Entry> {
        self.drawn().into_iter().filter(|e| !e.leaving()).collect()
    }

    /// Whether the column has anything to show (the surface is mapped while it does).
    pub fn showing(&self) -> Showing {
        match self.drawn().is_empty() {
            true => Showing::Nothing,
            false => Showing::Cards,
        }
    }

    /// When the earliest hold runs out, if any card is holding.
    pub fn next_wake(&self) -> Option<Stamp> {
        self.entries
            .iter()
            .filter_map(|e| match e.banner {
                Banner::Shown {
                    deadline: Deadline::At(at),
                } => Some(at),
                _ => None,
            })
            .min()
    }
}

/// Whether any card is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Showing {
    Nothing,
    Cards,
}

/// `deck` after `input` at `now`.
pub fn advance(deck: Deck, input: DeckIn, now: Stamp, p: &BannerParams) -> (Deck, Vec<DeckOut>) {
    let mut out = Vec::new();
    let (keyed, input) = match input {
        DeckIn::Cards(cards) => return super::deck_cards::follow(deck, cards, now, p),
        DeckIn::Risen(key) => (Some(key), BannerIn::Entered),
        DeckIn::Card(key, input) => (Some(key), input),
        DeckIn::Tick => (None, BannerIn::Tick),
    };
    let entries = deck
        .entries
        .into_iter()
        .map(|e| match keyed.is_none_or(|key| key == e.key) {
            true => feed(e, input, now, p, &mut out, Speak::Yes),
            false => e,
        })
        .filter(|e| e.banner != Banner::Gone)
        .collect();
    (Deck { entries, ..deck }, out)
}

/// Whether a card's leaving is told to the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Speak {
    Yes,
    /// The service dropped it itself.
    No,
}

/// `e` after `input`, its effects appended to `out`. A card the pointer is over is hovered again
/// as soon as it is shown (after it rose, or after a newer notification restarted its hold).
pub(super) fn feed(
    e: Entry,
    input: BannerIn,
    now: Stamp,
    p: &BannerParams,
    out: &mut Vec<DeckOut>,
    speak: Speak,
) -> Entry {
    let pointer = match input {
        BannerIn::HoverIn => Pointer::Over,
        BannerIn::HoverOut => Pointer::Away,
        _ => e.pointer,
    };
    let e = apply(Entry { pointer, ..e }, input, now, p, out, speak);
    match (e.pointer, e.banner, input) {
        (Pointer::Over, Banner::Shown { .. }, other) if other != BannerIn::HoverIn => {
            apply(e, BannerIn::HoverIn, now, p, out, speak)
        }
        _ => e,
    }
}

/// `e` after one machine step, its leaving told to the service.
fn apply(
    e: Entry,
    input: BannerIn,
    now: Stamp,
    p: &BannerParams,
    out: &mut Vec<DeckOut>,
    speak: Speak,
) -> Entry {
    let (banner, said) = step(e.banner, input, now, Persistence::of(e.card.life), p);
    for said in said {
        if let (BannerOut::Leave(why), Speak::Yes) = (said, speak) {
            out.extend(leave_commands(&e.card, why).into_iter().map(DeckOut::Send));
        }
    }
    Entry { banner, ..e }
}

/// A new entry for `card`, entering from `now`; the deck's next key.
pub(super) fn arrive(deck: &mut Deck, card: Card, now: Stamp, out: &mut Vec<DeckOut>) {
    let key = EntryKey(deck.next);
    deck.next = deck.next.wrapping_add(1);
    deck.entries.push(Entry {
        key,
        card,
        banner: Banner::Entering { since: now },
        pointer: Pointer::Away,
    });
    out.push(DeckOut::Rise(key));
}

#[cfg(test)]
#[path = "deck_tests.rs"]
mod tests;
