//! A helper's work in a terminal: whether the card offers to hand it to one, says it is in one,
//! or offers to bring it back, and the words on those buttons.

use companion_wire::{Handover, HelperCardView, RunsIn};

/// The button that hands a helper's work to a terminal.
pub const OPEN_IN_TERMINAL: &str = "Open in Terminal";

/// What the card says while the work is in a terminal.
pub const IN_TERMINAL: &str = "In Terminal";

/// The button that takes the work back from the terminal.
pub const BRING_BACK: &str = "Bring Back";

/// What the card shows about where the helper's work is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoverShown {
    /// Nothing: the work cannot move, or is on the desktop and stays there.
    Nothing,
    /// The work is on the desktop and can go to a terminal: [`OPEN_IN_TERMINAL`].
    OffersTerminal,
    /// The work is in a terminal: [`IN_TERMINAL`] with [`BRING_BACK`].
    InTerminal,
}

/// Where the card says the helper's work is, and what it offers.
pub fn handover_of(view: &HelperCardView) -> HandoverShown {
    match (&view.runs_in, &view.handover) {
        (RunsIn::Terminal, _) => HandoverShown::InTerminal,
        (RunsIn::Desktop, Handover::ToTerminal) => HandoverShown::OffersTerminal,
        (RunsIn::Desktop, Handover::Unavailable | Handover::BringBack) => HandoverShown::Nothing,
    }
}
