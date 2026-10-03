//! The terminal's standing grants ("allow from the terminal: this action, until logout") and the
//! breaker that pauses a terminal session, as the control center shows and changes them.

mod model;
mod step;
#[cfg(test)]
mod tests;

pub use model::{Breaker, ControlReach, Terminal, grant_name, grant_words, parse_grant};
pub use step::{TerminalEvent, step};
