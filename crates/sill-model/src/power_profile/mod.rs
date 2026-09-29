//! The power profile: power-profiles-daemon's words, as the control center draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{ProfileCommand, ProfileEvent};
pub use model::{PowerProfiles, Profile, ProfileDegraded};
pub use step::step;
