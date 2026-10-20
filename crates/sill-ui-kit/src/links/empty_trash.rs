//! The Empty Trash question's channels. The daemon opens the question on an output (the dock's
//! Empty Trash row asks for the action); every output's surface follows it; the surface that
//! shows it writes the person's yes, and the daemon empties the Trash.

use shell_host::OutputId;
use tokio::sync::watch;

use super::{Opening, Outbox, PanelLink, Reveal, Sent};

/// One opening of the question: where, and which opening it is. It carries nothing of its own.
pub type EmptyTrashOpening = Opening<()>;

/// Whether the question is up, remembering the last opening's serial.
pub type EmptyTrashShown = Reveal<EmptyTrashOpening>;

/// The person said to erase the Trash for good.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyTrashYes;

/// A yes, numbered.
pub type EmptyTrashAnswered = Sent<EmptyTrashYes>;

/// The shared channels, a root context of the question's surfaces and kept by the daemon. Clone
/// is cheap.
#[derive(Debug, Clone, Default)]
pub struct EmptyTrashLink {
    panel: PanelLink<()>,
    answered: Outbox<EmptyTrashYes>,
}

impl EmptyTrashLink {
    /// Closed, nothing answered.
    pub fn new() -> EmptyTrashLink {
        EmptyTrashLink::default()
    }

    /// The current value.
    pub fn now(&self) -> EmptyTrashShown {
        self.panel.now()
    }

    /// Ask on `output`: asking where it is already up stays the same opening.
    pub fn open(&self, output: OutputId) {
        self.panel.open(output, ());
    }

    /// Take the question down: Cancel, Escape.
    pub fn close(&self) {
        self.panel.close();
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<EmptyTrashShown> {
        self.panel.subscribe()
    }

    /// The person chose Empty Trash: the question comes down and the yes is sent.
    pub fn confirm(&self) {
        self.answered.send(EmptyTrashYes);
        self.panel.close();
    }

    /// The latest yes.
    pub fn last_answered(&self) -> Option<EmptyTrashAnswered> {
        self.answered.latest()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_answered(&self) -> watch::Receiver<Option<EmptyTrashAnswered>> {
        self.answered.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::EmptyTrashLink;
    use crate::links::OpenSerial;

    #[test]
    fn a_yes_counts_on_and_takes_the_question_down_and_a_cancel_sends_nothing() {
        let link = EmptyTrashLink::new();
        link.open(OutputId(1));
        assert_eq!(link.now().serial(), OpenSerial(1));
        link.close();
        assert_eq!(link.last_answered(), None);
        link.open(OutputId(1));
        link.confirm();
        link.open(OutputId(1));
        link.confirm();
        assert_eq!(link.last_answered().map(|sent| sent.seq), Some(2));
        assert!(link.now().opening().is_none());
    }
}
