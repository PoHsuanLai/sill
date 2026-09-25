//! The Wi-Fi page: the radio `Toggle` in its header, then `listed_networks` as rows (signal
//! glyph by strength, name, [`crate::surfaces::control_center::network_detail`], a lock for a
//! secured one); choosing a saved network sends `WifiCommand::Connect`, a new one opens COSMIC
//! Settings' Wi-Fi page (no password prompt yet), the connected one `Disconnect`. Sends
//! `WifiCommand::Scan` when it opens. "Wi-Fi Settings…" at the foot.

use dioxus::prelude::*;

/// The Wi-Fi page.
#[component]
pub fn WifiNetworks(on_back: EventHandler<()>) -> Element {
    let _ = on_back;
    todo!("M5 fill: control-center (c)")
}
