//! The shell's own actions, as `org.quire.Shell` declares them (`dist/intents/org.quire.Shell.toml`)
//! and intentd calls them (`IntentProvider1.Perform`): what an invocation of each means, and what
//! an answer says. The daemon turns a [`ShellAction`] into the same request `sill <verb>` makes.

mod action;
mod invocation;
#[cfg(test)]
mod tests;

pub use action::{ShellAction, ShellFault, WorkspaceStep};
pub use invocation::{action_of, outcome_of, undo_of};
