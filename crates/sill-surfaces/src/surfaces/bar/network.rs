//! The network status item and its menu (connection, connectivity).

use dioxus::prelude::*;
use shell_host::OutputInfo;

use super::menus::BarMenu;
use super::status::{network_icon, network_lines};
use super::status_item::StatusIcon;
use crate::hooks::use_network;

/// The network status item and its menu (connection, connectivity).
#[component]
pub fn NetworkItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    let network = use_network()();
    let [link, _] = network_lines(&network);
    rsx! {
        StatusIcon { menu: BarMenu::Network, icon: network_icon(&network), tooltip: link }
    }
}
