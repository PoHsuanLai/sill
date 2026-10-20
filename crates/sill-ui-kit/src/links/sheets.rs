//! Which output shows the sheets: the questions that ask the person something (polkit, confirm,
//! voice consent, accounts, capture, permission, the display trial). macOS asks on the screen
//! the person is working on, so a sheet opens on the focused window's output, else the first
//! output (the daemon does not know the pointer's).
//!
//! The daemon writes it and holds it still while any question is up, so a sheet does not follow
//! the focus: with a sheet that takes clicks on its card only, a click beside it changes the
//! focus, and the question must stay where it was put. Between bursts of questions the link
//! names no output. The latch that decides it is `sill::sheet_latch`.

use shell_host::OutputId;
use tokio::sync::watch;

use super::Shared;

/// The output the sheets show on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SheetOutput {
    /// No question is up (or no output yet).
    #[default]
    Nowhere,
    On(OutputId),
}

impl SheetOutput {
    /// Whether `output`'s sheet surface is the one that shows.
    pub fn shows_on(self, output: OutputId) -> bool {
        self == SheetOutput::On(output)
    }
}

/// The daemon's half and every sheet surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct SheetLink {
    output: Shared<SheetOutput>,
}

impl SheetLink {
    /// Nowhere yet.
    pub fn new() -> SheetLink {
        SheetLink::default()
    }

    /// Show the sheets on `output` (a repeat of the current one notifies nobody).
    pub fn set(&self, output: SheetOutput) {
        self.output.set(output);
    }

    /// Where they show now.
    pub fn now(&self) -> SheetOutput {
        self.output.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<SheetOutput> {
        self.output.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use crate::links::{SheetLink, SheetOutput};
    use shell_host::OutputId;

    #[test]
    fn one_output_shows_and_a_repeat_is_not_news() {
        let link = SheetLink::new();
        let mut rx = link.subscribe();
        assert!(!link.now().shows_on(OutputId(1)));
        link.set(SheetOutput::On(OutputId(1)));
        assert!(rx.has_changed().unwrap_or(false));
        assert!(rx.borrow_and_update().shows_on(OutputId(1)));
        assert!(!link.now().shows_on(OutputId(2)));
        link.set(SheetOutput::On(OutputId(1)));
        assert!(!rx.has_changed().unwrap_or(true));
    }
}
