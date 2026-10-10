//! Wi-Fi: NetworkManager's wireless radio, its access points and its saved networks
//! (design/20 §1.5: the control center's Wi-Fi module and its network list).
//!
//! A service of its own beside [`super::NetworkService`] rather than more fields on `Network`: the
//! bar draws `Network` and would repaint on every scan otherwise, and the two share nothing but the
//! bus. Same proxies as the rest of the network (sill's own, in `nm`).

mod backend;

use sill_model::wifi::{Wifi, WifiCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// NetworkManager's wireless radio as a [`Service`]. Without NetworkManager or a wireless device
/// the state stays at `Wifi::default()` (absent), and the control center hides the module's list.
#[derive(Debug)]
pub enum WifiService {}

impl Service for WifiService {
    const NAME: &'static str = "wifi";
    type State = Wifi;
    type Command = WifiCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
