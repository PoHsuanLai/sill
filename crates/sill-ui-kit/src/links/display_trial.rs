//! The display trial's two channels: the daemon writes the trial the compositor opens (a program
//! changed the displays and the compositor asks whether to keep them) and the banners' output's
//! trial surface follows it; the surface writes the person's answer and the daemon carries it
//! out. The compositor ends a trial by itself (answered, timed out, replaced) and the daemon
//! takes the question down on the link then. The link holds the banners' link too, so the
//! question shows on the output the other questions show on.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sill_model::desktop::{DisplayTrial, TrialAnswer, TrialId};
use tokio::sync::watch;

use super::BannerLink;

/// Counts the compositor's sends of a trial; each is one more than the last, across trials, so
/// a restart is told from the trial before it even when both carry one id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TrialSerial(pub u64);

/// The trial that is open now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialRun {
    /// Which send of a trial it is; a restart is a new one.
    pub serial: TrialSerial,
    /// The trial and the time it gave, counted from this send.
    pub trial: DisplayTrial,
}

/// What the person said about the trial `trial`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialAnswered {
    /// Counts up from 1 per link.
    pub seq: u64,
    /// The trial answered.
    pub trial: TrialId,
    /// The answer.
    pub answer: TrialAnswer,
}

/// The shared channels, a root context of the trial surfaces and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct DisplayTrialLink {
    open: watch::Sender<Option<TrialRun>>,
    answered: watch::Sender<Option<TrialAnswered>>,
    sent: Arc<AtomicU64>,
    banners: BannerLink,
}

impl DisplayTrialLink {
    /// No trial, no answer; the question shows on the output `banners` names.
    pub fn new(banners: BannerLink) -> DisplayTrialLink {
        DisplayTrialLink {
            open: watch::Sender::new(None),
            answered: watch::Sender::new(None),
            sent: Arc::new(AtomicU64::new(0)),
            banners,
        }
    }

    /// The compositor opened `trial`, or restarted the trial it has open: the question shows
    /// with the time it gives, counted from now. It replaces any trial before it.
    pub fn begin(&self, trial: DisplayTrial) {
        let serial = TrialSerial(self.sent.fetch_add(1, Ordering::Relaxed) + 1);
        self.open.send_replace(Some(TrialRun { serial, trial }));
    }

    /// The trial `trial` is over. A trial of another number stays.
    pub fn end(&self, trial: TrialId) {
        self.open.send_if_modified(|latest| {
            let ours = latest.as_ref().is_some_and(|run| run.trial.trial == trial);
            if ours {
                *latest = None;
            }
            ours
        });
    }

    /// The open trial, if any.
    pub fn latest(&self) -> Option<TrialRun> {
        *self.open.borrow()
    }

    /// A receiver for the surfaces to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<TrialRun>> {
        self.open.subscribe()
    }

    /// The output the question shows on: the banners'.
    pub fn banners(&self) -> &BannerLink {
        &self.banners
    }

    /// The person answered `trial`. The question comes down at once; the compositor's end of the
    /// trial arrives after and finds nothing open.
    pub fn answer(&self, trial: TrialId, answer: TrialAnswer) {
        self.answered.send_modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(TrialAnswered { seq, trial, answer });
        });
        self.end(trial);
    }

    /// The latest answer.
    pub fn last_answer(&self) -> Option<TrialAnswered> {
        *self.answered.borrow()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_answers(&self) -> watch::Receiver<Option<TrialAnswered>> {
        self.answered.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{DisplayTrial, TrialAnswer, TrialId};

    use super::{BannerLink, DisplayTrialLink, TrialSerial};

    fn trial(id: u32, seconds: u32) -> DisplayTrial {
        DisplayTrial {
            trial: TrialId::new(id),
            seconds,
        }
    }

    #[test]
    fn a_trial_opens_restarts_and_ends_with_its_own_number() {
        let link = DisplayTrialLink::new(BannerLink::new());
        assert_eq!(link.latest(), None);
        link.begin(trial(4, 15));
        let first = link.latest().expect("open");
        assert_eq!((first.serial, first.trial), (TrialSerial(1), trial(4, 15)));
        link.begin(trial(4, 15));
        let again = link.latest().expect("open");
        assert_eq!(
            again.serial,
            TrialSerial(2),
            "a restart is a send of its own"
        );
        link.end(TrialId::new(9));
        assert!(link.latest().is_some(), "another trial's end is not ours");
        link.end(TrialId::new(4));
        assert_eq!(link.latest(), None);
        link.begin(trial(4, 10));
        let next = link.latest().expect("open");
        assert_eq!(
            next.serial,
            TrialSerial(3),
            "the count goes on across trials"
        );
    }

    #[test]
    fn an_answer_counts_on_and_takes_the_question_down() {
        let link = DisplayTrialLink::new(BannerLink::new());
        assert_eq!(link.last_answer(), None);
        link.begin(trial(2, 15));
        link.answer(TrialId::new(2), TrialAnswer::Revert);
        assert_eq!(link.latest(), None);
        link.begin(trial(3, 15));
        link.answer(TrialId::new(3), TrialAnswer::Keep);
        let last = link.last_answer().expect("answered");
        assert_eq!(
            (last.seq, last.trial, last.answer),
            (2, TrialId::new(3), TrialAnswer::Keep)
        );
    }
}
