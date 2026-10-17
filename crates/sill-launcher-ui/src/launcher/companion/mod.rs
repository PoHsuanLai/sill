//! The panel's companion mode (design/32 section 4.4): the field is a prompt, the rows are the
//! companion's (the Ask row, its answers, recent activity; Tab visits Runs and Memory), and the
//! selected answer's card is the palette's `aside`.
//!
//! The mode is the opening's (`Opening::mode`): the summon opens it, the double tap switches it
//! in place. What the pane is showing is not stored: it follows the selected row
//! ([`pane_of`]). Everything here is pure; `events.rs` and `pane_keys.rs` call it.

mod ask;
mod command;
mod ctx;
mod keys;
mod pane;
#[cfg(test)]
mod tests;

pub use ask::ask_input;
pub use command::command_of;
pub use keys::{CompanionKey, CompanionOut, Selected, Way, companion_filter, companion_step, follows_state};
pub use pane::{Pane, pane_of, run_of, selected_of};
