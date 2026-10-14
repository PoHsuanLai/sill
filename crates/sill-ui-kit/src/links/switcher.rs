//! The daemon's asks to the switcher (`sill switcher next|prev|cancel|commit`, `sill debug
//! switcher`): one shared channel of the recent asks, numbered so two chords in a row are two
//! asks, and kept so that a second Tab arriving before the surface has read the first is still
//! a second step. Every output's switcher surface follows it and acts on asks for its own output.

use shell_host::OutputId;
use sill_launcher::AppId;
use tokio::sync::watch;

/// Which way a chord or key moves the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Next,
    Prev,
}

/// One ask.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwitcherAsk {
    /// The chord: the machine's `SwIn::Chord`.
    Chord(Dir),
    /// Close with no switch (the machine's Escape).
    Cancel,
    /// Switch and close (the machine's `ModifierReleased`).
    Commit,
    /// Open at once on these made-up apps (the current first); release, Q and H touch no window.
    Fake(Vec<AppId>),
}

/// Which ask it is, counting from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct AskSerial(pub u64);

/// The latest ask and where.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwitcherAsked {
    pub serial: AskSerial,
    pub output: OutputId,
    pub ask: SwitcherAsk,
}

/// How many asks the channel keeps: far more than a held chord sends between two reads.
const KEPT: usize = 16;

/// The recent asks, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Asks {
    recent: Vec<SwitcherAsked>,
}

impl Asks {
    /// The asks after `seen` (all of them for `None`), oldest first.
    pub fn since(&self, seen: Option<AskSerial>) -> Vec<SwitcherAsked> {
        self.recent
            .iter()
            .filter(|asked| seen.is_none_or(|serial| asked.serial > serial))
            .cloned()
            .collect()
    }

    /// The latest ask.
    pub fn latest(&self) -> Option<&SwitcherAsked> {
        self.recent.last()
    }

    fn push(&mut self, asked: SwitcherAsked) {
        self.recent.push(asked);
        let over = self.recent.len().saturating_sub(KEPT);
        self.recent.drain(..over);
    }
}

/// The daemon's half and every switcher surface's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct SwitcherLink {
    asked: watch::Sender<Asks>,
}

impl Default for SwitcherLink {
    fn default() -> Self {
        SwitcherLink::new()
    }
}

impl SwitcherLink {
    pub fn new() -> SwitcherLink {
        SwitcherLink {
            asked: watch::Sender::new(Asks::default()),
        }
    }

    /// Ask the switcher on `output`; every ask is new, even one equal to the last.
    pub fn ask(&self, output: OutputId, ask: SwitcherAsk) -> AskSerial {
        let mut serial = AskSerial::default();
        self.asked.send_modify(|asks| {
            serial = AskSerial(asks.latest().map_or(0, |last| last.serial.0) + 1);
            asks.push(SwitcherAsked {
                serial,
                output,
                ask,
            });
        });
        serial
    }

    /// The latest ask.
    pub fn latest(&self) -> Option<SwitcherAsked> {
        self.asked.borrow().latest().cloned()
    }

    pub fn subscribe(&self) -> watch::Receiver<Asks> {
        self.asked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use crate::links::{AskSerial, Dir, SwitcherAsk, SwitcherLink};

    #[test]
    fn every_ask_is_numbered_and_seen_even_when_repeated() {
        let link = SwitcherLink::new();
        let mut seen = link.subscribe();
        assert_eq!(link.latest(), None);
        let first = link.ask(OutputId(1), SwitcherAsk::Chord(Dir::Next));
        seen.mark_unchanged();
        let second = link.ask(OutputId(1), SwitcherAsk::Chord(Dir::Next));
        assert_eq!((first, second), (AskSerial(1), AskSerial(2)));
        assert_eq!(seen.has_changed().ok(), Some(true));
        assert_eq!(
            link.latest()
                .map(|asked| (asked.serial, asked.output, asked.ask)),
            Some((AskSerial(2), OutputId(1), SwitcherAsk::Chord(Dir::Next)))
        );
    }

    #[test]
    fn asks_made_between_two_reads_are_all_kept_in_order() {
        let link = SwitcherLink::new();
        let seen = link.ask(OutputId(1), SwitcherAsk::Chord(Dir::Next));
        link.ask(OutputId(1), SwitcherAsk::Chord(Dir::Next));
        link.ask(OutputId(1), SwitcherAsk::Commit);
        let fresh: Vec<_> = link
            .subscribe()
            .borrow()
            .since(Some(seen))
            .into_iter()
            .map(|asked| (asked.serial, asked.ask))
            .collect();
        assert_eq!(
            fresh,
            vec![
                (AskSerial(2), SwitcherAsk::Chord(Dir::Next)),
                (AskSerial(3), SwitcherAsk::Commit),
            ]
        );
        for _ in 0..40 {
            link.ask(OutputId(1), SwitcherAsk::Cancel);
        }
        assert_eq!(link.subscribe().borrow().since(None).len(), 16);
    }
}
