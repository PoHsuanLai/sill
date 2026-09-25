//! The Bluetooth page: the radio `Toggle` in its header, then `listed_devices` as rows (kind
//! glyph, name, [`crate::surfaces::control_center::device_detail`]); choosing a paired device
//! sends `BluetoothCommand::Connect` or `Disconnect`. Pairing a new device is COSMIC Settings'
//! ("Bluetooth Settings…" at the foot).

use dioxus::prelude::*;

/// The Bluetooth page.
#[component]
pub fn BluetoothDevices(on_back: EventHandler<()>) -> Element {
    let _ = on_back;
    todo!("M5 fill: control-center (c)")
}
