//! A control center module as its own bar item (`control_center.menu_bar_*`): a status icon
//! whose dropdown is the module's own component (`PopupBody::Module`). Replaces the pre-M5
//! volume, network and battery items, whose menus become the Sound, Wi-Fi and Battery modules.

use dioxus::prelude::*;
use shell_host::OutputInfo;
use sill_settings::ControlCenterModule;

/// `module`'s bar item on `output` (its glyph follows the module's state: signal strength,
/// volume level, battery level).
#[component]
pub fn ModuleItem(output: OutputInfo, module: ControlCenterModule) -> Element {
    let _ = (output, module);
    todo!("M5 fill: control-center (c)")
}
