//! Wi-Fi: NetworkManager's wireless radio, its access points and its saved networks
//! (design/20 §1.5: the control center's Wi-Fi module and its network list).
//!
//! A service of its own beside [`super::NetworkService`] rather than more fields on `Network`: the bar
//! draws `Network` and would repaint on every scan otherwise, and the two share nothing but the
//! bus. Same crate as the rest of the network (`cosmic-dbus-networkmanager`, MPL-2.0, linked,
//! never pasted from).

mod live;
mod model;

pub use model::{
    AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiEvent, listed_networks, step,
};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// What the Wi-Fi service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiCommand {
    /// `WirelessEnabled` on or off.
    Radio(Radio),
    /// `RequestScan` on the wireless device (the control center's list opening).
    Scan,
    /// Activate the saved connection for `ssid` on the wireless device. A network with no saved
    /// connection is dropped: joining a new network needs a password prompt, which is a later
    /// milestone's (it opens COSMIC Settings' Wi-Fi page meanwhile).
    Connect(Ssid),
    /// Deactivate the wireless device's active connection.
    Disconnect,
}

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
        live::run(env.buses.system.clone(), io.state, io.commands)
    }
}
