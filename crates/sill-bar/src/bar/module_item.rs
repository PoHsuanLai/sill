//! A control center module as its own bar item (`control_center.menu_bar_*`): the module's own
//! status icon, whose dropdown is the module's own component (`PopupBody::Module`).

use dioxus::prelude::*;
use sill_settings::ControlCenterModule;

use crate::control_center::modules;

/// `module`'s bar item: drawn as nothing for a module that cannot be one (`in_menu_bar` hides
/// those).
#[component]
pub fn ModuleItem(module: ControlCenterModule) -> Element {
    match modules::module(module).bar_item() {
        Some(face) => (face.item)(),
        None => rsx! {},
    }
}
