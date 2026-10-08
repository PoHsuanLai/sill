//! App launching: the requests, the scopes an app runs in, and how a desktop entry becomes an argv.

mod command;
mod model;
mod plan;
mod step;
#[cfg(test)]
mod tests;

pub use command::{
    LaunchCommand, LaunchOutcome, LaunchRequest, OpenRequest, SETTINGS_BUS_NAME, SettingsAccount,
};
pub use model::{LaunchedScope, Launches, ScopeUnit};
pub use plan::{launch_argv, scope_unit};
pub use step::step;
