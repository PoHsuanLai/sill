//! The dock's pinned apps, in order.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{PinAt, PinEvent};
pub use model::DockPins;
pub use step::step;
