//! App launching: the requests, the scopes an app runs in, and how a desktop entry becomes an argv.

mod command;
mod failure;
mod model;
mod plan;
mod settings_page;
mod step;
#[cfg(test)]
mod tests;
mod timing;

pub use command::{
    LaunchCommand, LaunchOutcome, LaunchRequest, OpenRequest, SETTINGS_BUS_NAME, SettingsAccount,
};
pub use failure::{FailureCause, LaunchNotice, failure_sentence, failure_text};
pub use model::{DONE_KEPT, LaunchedScope, Launches, ScopeUnit};
pub use plan::{launch_argv, scope_unit};
pub use settings_page::{SettingsPage, SettingsVisit};
pub use step::step;
pub use timing::{LaunchDone, LaunchTiming, Pid, Stamp, answer_to, launch_line};
