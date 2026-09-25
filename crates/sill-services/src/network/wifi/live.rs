//! NetworkManager's wireless side over the system bus: `WirelessEnabled` and
//! `WirelessHardwareEnabled`, the wireless device's access points and `LastScan`, the saved
//! connections' SSIDs, and the active access point; every change through
//! [`super::reduce_wifi`]; [`WifiCommand`]s carried out.

use tokio::sync::{mpsc, watch};

use super::WifiCommand;
use super::model::Wifi;

/// Runs until `commands` closes. Without a bus, NetworkManager or a wireless device, `tx` stays
/// at its default.
///
/// M5 freeze stub, filled by M5 agent (b) svc-media-wifi-focus (FINDINGS "M5 freeze"): until
/// then it returns at once, so the service is inert.
pub(super) async fn run(tx: watch::Sender<Wifi>, commands: mpsc::UnboundedReceiver<WifiCommand>) {
    let _ = (tx, commands);
}
