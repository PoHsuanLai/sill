//! The Wi-Fi tile: the radio as a quire `Toggle`, [`crate::surfaces::control_center::wifi_status`]
//! as its line, and a chevron to the network list (`Detail::Wifi`). Reads `use_wifi` and
//! `use_network`; sends `WifiCommand::Radio`.

use dioxus::prelude::*;

use crate::surfaces::control_center::page::Detail;

/// The Wi-Fi tile.
#[component]
pub fn WifiModule(on_detail: EventHandler<Detail>) -> Element {
    let _ = on_detail;
    todo!("M5 fill: control-center (c)")
}
