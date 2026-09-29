//! The battery: UPower's display device, as the bar draws it.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{BatteryCommand, BatteryEvent};
pub use model::{Battery, Charge, Remaining};
pub use step::step;
