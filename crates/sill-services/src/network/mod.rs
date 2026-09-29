//! The network: NetworkManager's primary connection over the system bus
//! (cosmic-dbus-networkmanager; MPL, link only).

mod live;
mod model;
pub mod wifi;

pub use model::{Activating, Connectivity, Link, Network, NetworkEvent, step};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// What the network service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Re-read the primary connection and connectivity.
    Refresh,
}

/// NetworkManager's primary connection as a [`Service`]. Without NetworkManager the state
/// stays at `Network::default()` (disconnected, connectivity unknown).
#[derive(Debug)]
pub enum NetworkService {}

impl Service for NetworkService {
    const NAME: &'static str = "network";
    type State = Network;
    type Command = NetworkCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        live::run(env.buses.system.clone(), io.state, io.commands)
    }
}
