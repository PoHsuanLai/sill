//! Where IBus's own bus is, chosen once at the edge.

use std::path::PathBuf;

use super::Dirs;

/// How IBus's bus is reached (it keeps its own, apart from the session bus).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum IbusRoute {
    /// The address files IBus writes in this folder (`<config>/ibus/bus`), newest first.
    Files(PathBuf),
    /// This address, whatever the files say (`IBUS_ADDRESS`, a test's private bus).
    Address(String),
    /// No IBus to look for.
    #[default]
    Absent,
}

impl IbusRoute {
    /// The process's own: `IBUS_ADDRESS` when it is set, else the person's address files.
    pub fn from_process() -> IbusRoute {
        match std::env::var("IBUS_ADDRESS") {
            Ok(address) if !address.is_empty() => IbusRoute::Address(address),
            _ => IbusRoute::Files(Dirs::from_process().config.join("ibus").join("bus")),
        }
    }
}
