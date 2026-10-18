//! The Split View picker's two channels: the daemon writes the offer the compositor makes (the
//! first window and its half, then each window that can fill the other) and every output's
//! picker surface follows it; the surface that shows the picker writes the person's answer, and
//! the daemon carries it out. The compositor ends an offer by itself (a pick, a cancel, the
//! first window leaving full screen), and the daemon closes it on the link then.

use sill_model::desktop::{SplitCard, SplitSide};
use tokio::sync::watch;

use super::Shared;

/// Counts offers; each is one more than the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OfferSerial(pub u64);

/// One Split View being chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitOffer {
    /// Which offer it is.
    pub serial: OfferSerial,
    /// The window already full screen, by its identifier.
    pub first: String,
    /// The half it took.
    pub side: SplitSide,
    /// The output it is on; empty when the desktop does not say.
    pub output: String,
    /// The windows that can fill the other half, most recently used first.
    pub cards: Vec<SplitCard>,
}

/// What the person did with an offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitAnswer {
    /// Picked this window (by its identifier) for the other half.
    Picked(String),
    /// Turned the choice down.
    Cancelled,
}

/// An answer to the offer of `first`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitAnswered {
    /// Counts up from 1 per link.
    pub seq: u64,
    /// The first window of the offer answered.
    pub first: String,
    /// The answer.
    pub answer: SplitAnswer,
}

/// The shared channels, a root context of every picker surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct SplitPickerLink {
    offer: Shared<Option<SplitOffer>>,
    answered: Shared<Option<SplitAnswered>>,
}

impl SplitPickerLink {
    /// No offer, no answer.
    pub fn new() -> SplitPickerLink {
        SplitPickerLink::default()
    }

    /// A new offer for the Split View of `first` on `output`, with no windows on it yet. It
    /// replaces any offer before it.
    pub fn begin(&self, first: impl Into<String>, side: SplitSide, output: impl Into<String>) {
        let (first, output) = (first.into(), output.into());
        self.offer.modify(|latest| {
            let serial = OfferSerial(latest.as_ref().map_or(0, |last| last.serial.0) + 1);
            *latest = Some(SplitOffer {
                serial,
                first,
                side,
                output,
                cards: Vec::new(),
            });
        });
    }

    /// One more window on the open offer, after the ones already on it. Nothing without an
    /// offer.
    pub fn add(&self, card: SplitCard) {
        self.offer.modify(|latest| {
            if let Some(offer) = latest {
                offer.cards.push(card);
            }
        });
    }

    /// The offer of `first` is over. An offer of another first window stays.
    pub fn end(&self, first: &str) {
        self.offer.modify_if(|latest| {
            let ours = latest.as_ref().is_some_and(|offer| offer.first == first);
            if ours {
                *latest = None;
            }
            ours
        });
    }

    /// The open offer, if any.
    pub fn latest(&self) -> Option<SplitOffer> {
        self.offer.now()
    }

    /// A receiver for the surfaces to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<SplitOffer>> {
        self.offer.subscribe()
    }

    /// The person answered the offer of `first`.
    pub fn answer(&self, first: impl Into<String>, answer: SplitAnswer) {
        let first = first.into();
        self.answered.modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(SplitAnswered { seq, first, answer });
        });
    }

    /// The latest answer.
    pub fn last_answer(&self) -> Option<SplitAnswered> {
        self.answered.now()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_answers(&self) -> watch::Receiver<Option<SplitAnswered>> {
        self.answered.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{OfferSerial, SplitAnswer, SplitPickerLink};
    use sill_model::desktop::{SplitCard, SplitSide};

    fn card(window: &str) -> SplitCard {
        SplitCard {
            window: window.to_owned(),
            app: "term".to_owned(),
            title: window.to_owned(),
        }
    }

    #[test]
    fn an_offer_gathers_its_windows_and_ends_with_its_first_window() {
        let link = SplitPickerLink::new();
        assert_eq!(link.latest(), None);
        link.add(card("w9"));
        assert_eq!(link.latest(), None, "no windows on no offer");
        link.begin("w1", SplitSide::Left, "DP-1");
        link.add(card("w2"));
        link.add(card("w3"));
        let offer = link.latest().expect("offered");
        assert_eq!(offer.serial, OfferSerial(1));
        assert_eq!((offer.first.as_str(), offer.side), ("w1", SplitSide::Left));
        assert_eq!(offer.output, "DP-1");
        let windows: Vec<&str> = offer.cards.iter().map(|c| c.window.as_str()).collect();
        assert_eq!(windows, ["w2", "w3"]);
        link.end("w7");
        assert!(link.latest().is_some(), "another window's end is not ours");
        link.end("w1");
        assert_eq!(link.latest(), None);
    }

    #[test]
    fn a_second_offer_replaces_the_first_and_counts_on() {
        let link = SplitPickerLink::new();
        link.begin("w1", SplitSide::Left, "");
        link.add(card("w2"));
        link.begin("w3", SplitSide::Right, "");
        let offer = link.latest().expect("offered");
        assert_eq!((offer.serial, offer.first.as_str()), (OfferSerial(2), "w3"));
        assert!(offer.cards.is_empty(), "the old windows are gone");
    }

    #[test]
    fn each_answer_counts_on() {
        let link = SplitPickerLink::new();
        assert_eq!(link.last_answer(), None);
        link.answer("w1", SplitAnswer::Cancelled);
        link.answer("w1", SplitAnswer::Picked("w2".to_owned()));
        let last = link.last_answer().expect("answered");
        assert_eq!(
            (last.seq, last.first.as_str(), last.answer),
            (2, "w1", SplitAnswer::Picked("w2".to_owned()))
        );
    }
}
