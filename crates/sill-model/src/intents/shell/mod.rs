//! The shell's own actions, as `org.quire.Shell` declares them (`dist/intents/org.quire.Shell.toml`)
//! and intentd calls them (`IntentProvider1.Perform`): what an invocation of each means, and what
//! an answer says. The daemon turns a [`ShellAction`] into the same request `sill <verb>` makes.

mod action;
mod invocation;
mod listing;
mod menu;
#[cfg(test)]
mod tests;
mod window_ask;
mod window_plan;
#[cfg(test)]
mod window_tests;

pub use action::{ShellAction, ShellFault};
pub use invocation::{action_of, outcome_of, undo_of};
pub use listing::LISTING_MAX;
pub use menu::{MenuAsk, menu_ask_of};
pub use window_ask::{WindowAsk, WorkspaceRef, window_ask_of};
pub use window_plan::{WindowAnswer, WindowFault, plan, window_by_id};
