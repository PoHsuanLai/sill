//! The column's cards over time, pure: which cards are drawn (the [`stack`](super::stack)'s,
//! plus the ones still sliding out), one banner [`machine`](super::machine) per card, where each
//! card sits (off the edge, at rest, following a drag), and what the column must do next (start
//! an entrance or an exit, tell the service). The view (`drive.rs`) owns the timers and the
//! pointer; everything it decides with is here, tested by table (`deck_tests.rs`).
//!
//! A card keeps its entry while its app's banners keep coming: a newer notification replaces
//! the content, bumps the count and starts the hold again (design/13 §13.3.6 "Grouping by app").
//! A card the service dropped by itself (`CloseNotification`, a replacement elsewhere, Do Not
//! Disturb's Dismiss All) leaves without telling the service anything.

use sill_services::{Cpx, NotificationsCommand, Stamp};
use sill_settings::{NotificationSwipe, NotificationsSettings};

use super::commands::leave_commands;
use super::machine::{
    Banner, BannerIn, BannerOut, BannerParams, Deadline, Leave, Persistence, step,
};
use super::stack::Card;
use crate::surfaces::notification_card::Expansion;

/// The deck's numbers and choices, from `notifications.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckParams {
    pub banner: BannerParams,
    /// What a swipe (or a drag past the threshold) tells the service.
    pub swipe: NotificationSwipe,
}

impl DeckParams {
    pub fn new(n: &NotificationsSettings) -> DeckParams {
        DeckParams {
            banner: BannerParams::new(n),
            swipe: n.swipe,
        }
    }
}

/// What the service hears when a card leaves for `why` after `input`: a swipe under
/// `notifications.swipe = "keep_in_center"` (macOS: a swiped banner stays in Notification Center)
/// is said as a timeout, which takes the banner away and keeps the notification; the close
/// button, and a swipe under `"dismiss"`, dismiss it.
pub fn told(why: Leave, input: BannerIn, swipe: NotificationSwipe) -> Leave {
    match (why, input, swipe) {
        (
            Leave::Dismissed,
            BannerIn::DragEnd { .. } | BannerIn::Swipe { .. },
            NotificationSwipe::KeepInCenter,
        ) => Leave::Timeout,
        (why, _, _) => why,
    }
}

/// One drawn card's identity for its life on screen (a new one per arrival).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryKey(pub u64);

/// Whether the pointer is over a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pointer {
    Over,
    #[default]
    Away,
}

/// Where a card is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// Off the output's edge: before its entrance starts, and once its exit has.
    Offstage,
    /// In its place.
    Rest,
    /// Following a drag this far (already damped).
    Follow(Cpx),
}

/// One drawn card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: EntryKey,
    pub card: Card,
    pub banner: Banner,
    pub pointer: Pointer,
    pub pose: Pose,
    pub expansion: Expansion,
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
    next: u64,
}

/// What reaches the deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckIn {
    /// The stack the service's banners make now (this output's; none elsewhere).
    Cards(Vec<Card>),
    /// A frame has passed since the card mounted off the edge: its entrance can start.
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
    /// Answer with `BannerIn::Entered` once the entrance has played.
    Entrance(EntryKey),
    /// Answer with `BannerIn::Left` once the exit has played.
    Exit(EntryKey),
    /// Tell the service.
    Send(NotificationsCommand),
}

impl Deck {
    /// The cards to draw, newest first (the order the stack shows them in, top down).
    pub fn drawn(&self) -> Vec<&Entry> {
        let mut drawn: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| e.banner != Banner::Gone)
            .collect();
        drawn.sort_by_key(|e| std::cmp::Reverse(e.card.newest()));
        drawn
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
pub fn advance(deck: Deck, input: DeckIn, now: Stamp, p: &DeckParams) -> (Deck, Vec<DeckOut>) {
    match input {
        DeckIn::Cards(cards) => super::deck_cards::follow(deck, cards, now, p),
        DeckIn::Risen(key) => {
            let known = deck.entries.iter().any(|e| e.key == key && !e.leaving());
            let out = known.then_some(DeckOut::Entrance(key)).into_iter().collect();
            (
                with_entry(deck, key, |e| Entry {
                    pose: Pose::Rest,
                    ..e
                }),
                out,
            )
        }
        DeckIn::Card(key, input) => {
            let mut out = Vec::new();
            let entries = deck
                .entries
                .into_iter()
                .map(|e| match e.key == key {
                    true => feed(e, input, now, p, &mut out, Speak::Yes),
                    false => e,
                })
                .filter(|e| e.banner != Banner::Gone)
                .collect();
            (Deck { entries, ..deck }, out)
        }
        DeckIn::Tick => {
            let mut out = Vec::new();
            let entries = deck
                .entries
                .into_iter()
                .map(|e| feed(e, BannerIn::Tick, now, p, &mut out, Speak::Yes))
                .collect();
            (Deck { entries, ..deck }, out)
        }
    }
}

/// Whether a card's leaving is told to the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Speak {
    Yes,
    /// The service dropped it itself.
    No,
}

/// `e` after `input`, its effects appended to `out`. A card the pointer is over is hovered again
/// as soon as it is shown (after its entrance, or a drag that sprang back).
pub(super) fn feed(
    e: Entry,
    input: BannerIn,
    now: Stamp,
    p: &DeckParams,
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

/// `e` after one machine step, its outputs turned into the entry's pose and the deck's effects.
fn apply(
    e: Entry,
    input: BannerIn,
    now: Stamp,
    p: &DeckParams,
    out: &mut Vec<DeckOut>,
    speak: Speak,
) -> Entry {
    let (banner, said) = step(e.banner, input, now, Persistence::of(e.card.life), &p.banner);
    said.into_iter().fold(Entry { banner, ..e }, |e, said| match said {
        BannerOut::RequestTick(_) => e,
        BannerOut::Expand => Entry {
            expansion: Expansion::Expanded,
            ..e
        },
        BannerOut::Collapse => Entry {
            expansion: Expansion::Collapsed,
            ..e
        },
        BannerOut::Offset(dx) => Entry {
            pose: Pose::Follow(dx),
            ..e
        },
        BannerOut::SpringBack => Entry {
            pose: Pose::Rest,
            ..e
        },
        BannerOut::Leave(why) => {
            if speak == Speak::Yes {
                let why = told(why, input, p.swipe);
                out.extend(leave_commands(&e.card, why).into_iter().map(DeckOut::Send));
            }
            out.push(DeckOut::Exit(e.key));
            Entry {
                pose: Pose::Offstage,
                expansion: Expansion::Collapsed,
                ..e
            }
        }
        BannerOut::Remove => e,
    })
}

/// `deck` with `key`'s entry changed by `f`.
fn with_entry(deck: Deck, key: EntryKey, f: impl Fn(Entry) -> Entry) -> Deck {
    let entries = deck
        .entries
        .into_iter()
        .map(|e| if e.key == key { f(e) } else { e })
        .collect();
    Deck { entries, ..deck }
}

/// A new entry for `card`, off the edge, entering from `now`; the deck's next key.
pub(super) fn arrive(deck: &mut Deck, card: Card, now: Stamp, out: &mut Vec<DeckOut>) {
    let key = EntryKey(deck.next);
    deck.next += 1;
    deck.entries.push(Entry {
        key,
        card,
        banner: Banner::Entering { since: now },
        pointer: Pointer::Away,
        pose: Pose::Offstage,
        expansion: Expansion::Collapsed,
    });
    out.push(DeckOut::Rise(key));
}

#[cfg(test)]
#[path = "deck_tests.rs"]
mod tests;
