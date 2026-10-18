//! Whether the notification center is open, and on which output: one shared state written by
//! every opener (the bar's clock, `sill notifications {toggle,open,close}`) and followed by
//! every output's center surface onto its own visibility, the way the power menu shares
//! `PowerMenuLink`.

use shell_host::OutputId;
use tokio::sync::watch;

use super::Shared;

/// The center's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CenterShown {
    #[default]
    Closed,
    Open(OutputId),
}

impl CenterShown {
    /// Whether `output`'s center is the open one.
    pub fn open_on(self, output: OutputId) -> bool {
        self == CenterShown::Open(output)
    }
}

/// The openers' half and every center surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct CenterLink {
    shown: Shared<CenterShown>,
}

impl CenterLink {
    /// Closed.
    pub fn new() -> CenterLink {
        CenterLink::default()
    }

    /// Open on `output` (moving it there if it is open elsewhere).
    pub fn open(&self, output: OutputId) {
        self.set(CenterShown::Open(output));
    }

    /// Close it, wherever it is.
    pub fn close(&self) {
        self.set(CenterShown::Closed);
    }

    /// Close it when open anywhere, else open it on `output`: the clock and the shortcut.
    pub fn toggle(&self, output: OutputId) {
        let next = match self.now() {
            CenterShown::Open(_) => CenterShown::Closed,
            CenterShown::Closed => CenterShown::Open(output),
        };
        self.set(next);
    }

    /// The state now.
    pub fn now(&self) -> CenterShown {
        self.shown.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<CenterShown> {
        self.shown.subscribe()
    }

    fn set(&self, next: CenterShown) {
        self.shown.set(next);
    }
}

#[cfg(test)]
mod tests {
    use crate::links::{CenterLink, CenterShown};
    use shell_host::OutputId;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn open_close_and_toggle() {
        let link = CenterLink::new();
        let mut rx = link.subscribe();
        link.toggle(ONE);
        assert_eq!(link.now(), CenterShown::Open(ONE));
        assert!(link.now().open_on(ONE));
        assert!(!link.now().open_on(TWO));
        link.toggle(TWO);
        assert_eq!(
            link.now(),
            CenterShown::Closed,
            "toggle closes wherever it is"
        );
        link.open(TWO);
        link.open(ONE);
        assert_eq!(link.now(), CenterShown::Open(ONE), "open moves it");
        assert!(rx.has_changed().unwrap_or(false));
        rx.borrow_and_update();
        link.open(ONE);
        assert!(!rx.has_changed().unwrap_or(true), "a repeat is not news");
        link.close();
        link.close();
        assert_eq!(*rx.borrow_and_update(), CenterShown::Closed);
    }
}
