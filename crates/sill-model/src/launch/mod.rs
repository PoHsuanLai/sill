//! App launching: the requests, the scopes an app runs in, and how a desktop entry becomes an argv.

mod command;
mod failure;
mod model;
mod plan;
mod settings_page;
mod step;
#[cfg(test)]
mod tests;

pub use command::{
    LaunchCommand, LaunchOutcome, LaunchRequest, OpenRequest, SETTINGS_BUS_NAME, SettingsAccount,
};
pub use failure::{FailureCause, LaunchNotice, failure_text};
pub use model::{LaunchedScope, Launches, ScopeUnit};
pub use plan::{launch_argv, scope_unit};
pub use settings_page::SettingsPage;
pub use step::step;
