//! The desktop widget layer (design/20 §1.14; macOS Sonoma's desktop widgets; M10 lane b): the
//! widgets `widgets.desktop_widgets` names, placed on a grid over the wallpaper and under every
//! window, shown while `widgets.desktop` is `On`. One surface per output
//! ([`layer::desktop_layer`]); its input and blur regions are the widgets' frames only, so a
//! click between them reaches the desktop below. Where each widget sits is saved state, per
//! output (`$XDG_STATE_HOME/sill/desktop-widgets.json`, [`saved`]), placed by
//! [`placement::place`]; a widget is moved by dragging it (FINDINGS "Widget drag": [`drag`],
//! [`arrange`], [`placed`], [`glide`]), plain pointer events on the layer, at the frame level:
//! nothing inside a widget's own view takes part. A widget taken away plays its exit before it
//! goes ([`leaving`]).

pub mod arrange;
pub mod drag;
pub mod geometry;
pub mod glide;
pub mod layer;
pub mod leaving;
pub mod placed;
pub mod placement;
pub mod saved;
mod view;

pub use layer::desktop_layer;
pub use placement::{GridSize, GridSlot, Placement, grid_size, place};
pub use saved::{OutputKey, PlacementFile};
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
