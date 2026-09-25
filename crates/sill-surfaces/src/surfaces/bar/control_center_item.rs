//! The control center's bar item: the last status item, just left of the clock (macOS's order:
//! status items, Control Center, clock), a quire `IconButton{Status}` that opens
//! `BarMenu::ControlCenter`, whose popup hangs from the output's right edge (`MenuSide::OutputEdge`)
//! rather than from the item.

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The control center's item on `output`.
#[component]
pub fn ControlCenterItem(output: OutputInfo) -> Element {
    let _ = output;
    todo!("M5 fill: control-center (c)")
}
