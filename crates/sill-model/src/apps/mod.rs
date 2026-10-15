//! The installed apps: desktop entries parsed, their exec lines expanded, and the index the
//! launcher and dock match against.

mod command;
mod exec;
mod index;
mod match_app;

pub use command::AppsCommand;
pub use exec::{ExecArgs, ExecError, expand_exec};
pub use index::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal, step,
};
pub use match_app::match_app;

/// A minimal entry with `id` and `name`, for the tests of what reads an index.
#[cfg(test)]
pub(crate) use index::tests::app as test_app;
