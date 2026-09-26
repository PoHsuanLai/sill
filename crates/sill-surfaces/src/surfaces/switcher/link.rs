//! The daemon's asks to the switcher (`sill switcher next|prev|cancel|commit`, `sill debug
//! switcher`): one shared channel of the latest ask, numbered so two chords in a row are two
//! asks. Every output's switcher surface follows it and acts on asks for its own output.
//! Implemented at the freeze: the daemon's wiring depends on it.

use shell_host::OutputId;
use sill_launcher::AppId;
use tokio::sync::watch;

use super::machine::Dir;

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

/// The daemon's half and every switcher surface's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct SwitcherLink {
    asked: watch::Sender<Option<SwitcherAsked>>,
}

impl Default for SwitcherLink {
    fn default() -> Self {
        SwitcherLink::new()
    }
}

impl SwitcherLink {
    pub fn new() -> SwitcherLink {
        SwitcherLink {
            asked: watch::Sender::new(None),
        }
    }

    /// Ask the switcher on `output`; every ask is new, even one equal to the last.
    pub fn ask(&self, output: OutputId, ask: SwitcherAsk) -> AskSerial {
        let mut serial = AskSerial::default();
        self.asked.send_modify(|now| {
            serial = AskSerial(now.as_ref().map_or(0, |last| last.serial.0) + 1);
            *now = Some(SwitcherAsked {
                serial,
                output,
                ask,
            });
        });
        serial
    }

    /// The latest ask.
    pub fn latest(&self) -> Option<SwitcherAsked> {
        self.asked.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<SwitcherAsked>> {
        self.asked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{AskSerial, SwitcherAsk, SwitcherLink};
    use crate::surfaces::switcher::machine::Dir;

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
}
