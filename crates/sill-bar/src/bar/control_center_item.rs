//! The control center's bar item: the last status item, just left of the clock (macOS's order:
//! status items, Control Center, clock), a quire `Button{Status}` that opens
//! `BarMenu::ControlCenter`, whose popup hangs from the output's right edge (`MenuSide::OutputEdge`)
//! rather than from the item.

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;

use super::menus::BarMenu;
use super::status_item::StatusIcon;

/// The control center's glyph: two switches, as macOS's.
const GLYPH: Icon = Icon::Switches;

/// The control center's item on `output`.
#[component]
pub fn ControlCenterItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    rsx! {
        StatusIcon {
            menu: BarMenu::ControlCenter,
            icon: GLYPH,
            tooltip: "Control Center".to_owned(),
        }
    }
}
