//! The Force Quit Applications panel's three channels. The daemon writes whether the panel is
//! open and on which output (`sill force-quit`, the keyboard shortcut) and which apps the desktop
//! lists, and every output's panel surface follows both; the surface that shows the panel writes
//! the app the person confirmed, and the daemon carries it out.

use shell_host::OutputId;
use sill_model::desktop::ForceQuitRow;
use tokio::sync::watch;

use super::{Opening, Outbox, PanelLink, Reveal, Sent, Shared};

/// One opening of the panel: where, and which opening it is (a new one starts afresh). It
/// carries nothing of its own.
pub type ForceQuitOpening = Opening<()>;

/// Whether the panel is open, remembering the last opening's serial so the next one counts on.
pub type ForceQuitShown = Reveal<ForceQuitOpening>;

/// An app the person confirmed force quitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceQuitAsk {
    /// The app's key, as `DesktopCommand::ForceQuitApp` quotes it.
    pub app: String,
}

/// A confirmation, numbered.
pub type ForceQuitAsked = Sent<ForceQuitAsk>;

/// The shared channels, a root context of every panel surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct ForceQuitLink {
    panel: PanelLink<()>,
    rows: Shared<Vec<ForceQuitRow>>,
    asked: Outbox<ForceQuitAsk>,
}

impl ForceQuitLink {
    /// Closed, no apps listed, nothing asked.
    pub fn new() -> ForceQuitLink {
        ForceQuitLink::default()
    }

    /// The current value.
    pub fn now(&self) -> ForceQuitShown {
        self.panel.now()
    }

    /// Open on `output`: opening an open panel where it is stays the same opening, anywhere
    /// else is a new one.
    pub fn open(&self, output: OutputId) {
        self.panel.open(output, ());
    }

    /// Open on `output` when closed, close when open: the shortcut.
    pub fn toggle(&self, output: OutputId) {
        self.panel.toggle(output, ());
    }

    /// Close: Escape, Done, a press away from the panel, or `sill force-quit close`.
    pub fn close(&self) {
        self.panel.close();
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<ForceQuitShown> {
        self.panel.subscribe()
    }

    /// The apps the desktop lists now.
    pub fn set_rows(&self, rows: Vec<ForceQuitRow>) {
        self.rows.set(rows);
    }

    /// A receiver for the apps the desktop lists.
    pub fn subscribe_rows(&self) -> watch::Receiver<Vec<ForceQuitRow>> {
        self.rows.subscribe()
    }

    /// The person confirmed force quitting `app`.
    pub fn ask(&self, app: impl Into<String>) {
        self.asked.send(ForceQuitAsk { app: app.into() });
    }

    /// The latest confirmation.
    pub fn last_asked(&self) -> Option<ForceQuitAsked> {
        self.asked.latest()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_asked(&self) -> watch::Receiver<Option<ForceQuitAsked>> {
        self.asked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{ForceQuitLink, ForceQuitShown};
    use crate::links::{OpenSerial, Reveal};

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn the_shortcut_opens_and_closes_and_each_opening_counts_on() {
        let closed = ForceQuitShown::default();
        let opened = closed.toggle(ONE, ());
        assert_eq!(opened.serial(), OpenSerial(1));
        assert_eq!(opened.opening().map(|o| o.output), Some(ONE));
        let shut = opened.toggle(ONE, ());
        assert_eq!(
            shut,
            Reveal::Hidden {
                last: OpenSerial(1)
            }
        );
        assert_eq!(shut.toggle(TWO, ()).serial(), OpenSerial(2));
    }

    #[test]
    fn opening_where_it_is_open_is_no_new_opening_but_elsewhere_is() {
        let opened = ForceQuitShown::default().open(ONE, ());
        assert_eq!(opened.open(ONE, ()), opened);
        assert_eq!(opened.open(TWO, ()).serial(), OpenSerial(2));
        assert_eq!(opened.open(TWO, ()).opening().map(|o| o.output), Some(TWO));
    }

    #[test]
    fn each_confirmation_counts_on() {
        let link = ForceQuitLink::new();
        assert_eq!(link.last_asked(), None);
        link.ask("term");
        link.ask("term");
        let last = link.last_asked().expect("asked");
        assert_eq!((last.seq, last.app.as_str()), (2, "term"));
    }
}
