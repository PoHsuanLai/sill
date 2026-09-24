//! The dock (design/20 §1.2, design/10, 06 §20.1): one per output, pinned apps then running
//! ones grouped by `match_app`, Plank magnification, bounces, labels, badges, clicks, drags and
//! auto-hide, driven by the pure [`machine`].

#![allow(unused_variables, dead_code)] // Freeze stubs

pub mod context_menu;
pub mod geometry;
pub mod items;
pub mod layer;
pub mod machine;
pub mod tile;

pub use geometry::magnify_params;
pub use items::{DockItem, ItemKind, dock_items};
pub use layer::dock_layer;

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The dock on `output`: `Ds` root with the output's SpaceLook, `Material::Dock` pill
/// (`dock-pill`), hit element (`dock-hit`), tiles laid out by `sill_services::magnify` from the
/// machine's `displayed` progress.
#[component]
pub fn Dock(output: OutputInfo) -> Element {
    todo!("M3 dock-ui")
}
