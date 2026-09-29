//! The default sink's volume and mute, as the bar and the OSD draw them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{Step, VolumeCommand, VolumeEvent};
pub use model::{Mute, Volume};
pub use step::{step, stepped};
