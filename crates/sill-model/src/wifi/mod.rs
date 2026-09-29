//! Wi-Fi: NetworkManager's wireless radio and the networks in range, as the control center lists
//! them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{WifiCommand, WifiEvent};
pub use model::{AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, listed_networks};
pub use step::step;
