//! Whether the notification center is open, and on which output: one shared state written by
//! every opener (the bar's clock, `sill notifications {toggle,open,close}`) and followed by
//! every output's center surface onto its own visibility, the way the power menu shares
//! `PowerMenuLink`. The center carries nothing of its own, so it is a bare [`PanelLink`].

use shell_host::{KeyboardFocus, OutputId};
use tokio::sync::watch;

use super::{Opening, PanelLink, Reveal};

/// One opening of the center.
pub type CenterOpening = Opening<()>;

/// The openers' half and every center surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct CenterLink {
    panel: PanelLink<()>,
}

impl CenterLink {
    /// Closed.
    pub fn new() -> CenterLink {
        CenterLink::default()
    }

    /// Open on `output` (moving it there if it is open elsewhere).
    pub fn open(&self, output: OutputId) {
        self.panel.open(output, ());
    }

    /// Close it, wherever it is.
    pub fn close(&self) {
        self.panel.close();
    }

    /// Close it when open anywhere, else open it on `output`: the clock and the shortcut.
    pub fn toggle(&self, output: OutputId) {
        self.panel.toggle(output, ());
    }

    /// The center's keyboard focus changed: losing it, after having it, closes the center.
    pub fn focus(&self, focus: KeyboardFocus) {
        self.panel.focus(focus);
    }

    /// The state now.
    pub fn now(&self) -> Reveal<CenterOpening> {
        self.panel.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Reveal<CenterOpening>> {
        self.panel.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use crate::links::CenterLink;
    use shell_host::OutputId;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn open_close_and_toggle() {
        let link = CenterLink::new();
        let mut rx = link.subscribe();
        link.toggle(ONE);
        assert!(link.now().open_on(ONE));
        assert!(!link.now().open_on(TWO));
        link.toggle(TWO);
        assert!(
            link.now().opening().is_none(),
            "toggle closes wherever it is"
        );
        link.open(TWO);
        link.open(ONE);
        assert!(link.now().open_on(ONE), "open moves it");
        assert!(rx.has_changed().unwrap_or(false));
        rx.borrow_and_update();
        link.open(ONE);
        assert!(!rx.has_changed().unwrap_or(true), "a repeat is not news");
        link.close();
        link.close();
        assert!(rx.borrow_and_update().opening().is_none());
    }
}
