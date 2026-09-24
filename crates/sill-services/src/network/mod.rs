//! The network: NetworkManager's primary connection over the system bus (plan hooks table,
//! `use_network`: "cosmic-dbus-networkmanager; MPL, link only").

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;

pub use model::{Connectivity, Link, Network, NetworkEvent, reduce_network};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the network service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Re-read the primary connection and connectivity.
    Refresh,
}

/// Start the network task on `rt`. Without NetworkManager the receiver stays at
/// `Network::default()` (disconnected, connectivity unknown).
pub fn spawn(rt: &Handle) -> (watch::Receiver<Network>, Control<NetworkCommand>) {
    todo!(
        "M1 svc-dbus: NetworkManager PrimaryConnection, Connectivity, AP Strength -> reduce_network"
    )
}
