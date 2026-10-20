//! The network: NetworkManager's primary connection over the system bus
//! (the `airwaves-nm` proxies).

mod backend;

use sill_model::network::{Network, NetworkCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

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
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
