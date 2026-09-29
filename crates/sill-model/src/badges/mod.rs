//! App badges: LauncherEntry counts, progress and urgency per app, as the dock draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{BadgeEvent, BadgeProp, BadgesCommand};
pub use model::{Badge, Badges, parse_app_uri};
pub use step::step;
