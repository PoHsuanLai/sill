//! The terminal's transition.

use docket_vocab::ActionRef;

use super::model::{Breaker, ControlReach, Terminal};

/// What happens to the terminal's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// The grants were read.
    Loaded(Vec<ActionRef>),
    /// intentd could not be read (not on the bus, or it refused).
    Unreachable,
    /// A grant was revoked.
    Revoked(ActionRef),
    /// The breaker paused a session, or `Control.Resume` lifted the pause.
    Breaker(Breaker),
}

/// `terminal` after `event`. Grants are kept sorted and without repeats.
pub fn step(terminal: Terminal, event: TerminalEvent) -> Terminal {
    match event {
        TerminalEvent::Loaded(mut grants) => {
            grants.sort();
            grants.dedup();
            Terminal {
                grants,
                reach: ControlReach::Reached,
                ..terminal
            }
        }
        // What was shown stays, said to be stale: the person can still revoke what they saw.
        TerminalEvent::Unreachable => Terminal {
            reach: ControlReach::Unreachable,
            ..terminal
        },
        TerminalEvent::Revoked(gone) => Terminal {
            grants: terminal.grants.into_iter().filter(|g| *g != gone).collect(),
            ..terminal
        },
        TerminalEvent::Breaker(breaker) => Terminal {
            breaker,
            ..terminal
        },
    }
}
