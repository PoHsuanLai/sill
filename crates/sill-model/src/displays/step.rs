//! The displays' pure transition.

use super::{Displays, DisplaysEvent};

/// `displays` after `event`: each source replaces its own part.
pub fn step(displays: Displays, event: DisplaysEvent) -> Displays {
    match event {
        DisplaysEvent::Outputs(outputs) => Displays {
            outputs,
            ..displays
        },
        DisplaysEvent::Ddc(ddc) => Displays { ddc, ..displays },
        DisplaysEvent::Backlights(backlights) => Displays {
            backlights,
            ..displays
        },
    }
}
