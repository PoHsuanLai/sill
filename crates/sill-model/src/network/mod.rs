//! The network: NetworkManager's primary connection and connectivity, as the bar draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{NetworkCommand, NetworkEvent};
pub use model::{Activating, Connectivity, Link, Network};
pub use step::step;
