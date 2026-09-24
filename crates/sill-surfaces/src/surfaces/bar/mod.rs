//! The bar (design/20 §1.1, design/13 §13.3.1-13.3.4, 06 §20.3): one per output, left the app
//! name and the workspace indicator, right tray, volume, network, battery, clock; every menu an
//! `xdg_popup` with a quire `Menu`, tracked by [`menu_track`].

#![allow(unused_variables, dead_code)] // Freeze stubs

pub mod app_name;
pub mod battery;
pub mod clock;
pub mod layer;
pub mod menu_track;
pub mod network;
pub mod popup;
pub mod tray;
pub mod volume;
pub mod workspaces;

pub use layer::bar_layer;

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The bar on `output`: `Ds` root with the workspace's SpaceLook, `Material::Bar`, the left
/// and right groups, and the one reusable menu popup.
#[component]
pub fn Bar(output: OutputInfo) -> Element {
    todo!("M1 bar-ui")
}
