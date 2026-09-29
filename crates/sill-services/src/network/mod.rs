//! The network: NetworkManager's primary connection over the system bus
//! (cosmic-dbus-networkmanager; MPL, link only).

mod live;
mod model;
pub mod wifi;

pub use model::{Activating, Connectivity, Link, Network, NetworkEvent, reduce_network};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::Bus;

/// What the network service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Re-read the primary connection and connectivity.
    Refresh,
}

/// Start the network task on `rt`. Without NetworkManager the receiver stays at
/// `Network::default()` (disconnected, connectivity unknown).
pub fn spawn(rt: &Handle, bus: Bus) -> (watch::Receiver<Network>, Control<NetworkCommand>) {
    let (tx, rx) = watch::channel(Network::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(bus, tx, commands));
    (rx, control)
}
