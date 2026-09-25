//! The desktop widget layer (design/20 §1.14; macOS Sonoma's desktop widgets; M10 lane b,
//! frozen as a skeleton): the widgets `widgets.desktop_widgets` names, placed on a grid over the
//! wallpaper and under every window, shown while `widgets.desktop` is `On`. One surface per
//! output ([`layer::desktop_layer`]); its input and blur regions are the widgets' frames only,
//! so a click between them reaches the desktop below. Where each widget sits is saved state
//! (`$XDG_STATE_HOME/sill/desktop-widgets.json`), placed by [`placement::place`]; dragging a
//! widget to another cell is later work (it wants a drag within the surface, which is plain
//! pointer events, not shell-host's G4).

pub mod geometry;
pub mod layer;
pub mod placement;
pub mod saved;
mod view;

pub use layer::desktop_layer;
pub use placement::{GridSize, GridSlot, Placement, grid_size, place};
pub use saved::PlacementFile;
pub use view::{WIDGET_RADIUS, widget_regions};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The layer's own stylesheet (the grid), injected beside `widgets_css`.
pub const DESKTOP_WIDGETS_CSS: &str = include_str!("../../style/desktop_widgets.css");

/// The desktop widgets on `output` (`view.rs`).
#[component]
pub fn DesktopWidgets(output: OutputInfo) -> Element {
    view::desktop(output)
}
