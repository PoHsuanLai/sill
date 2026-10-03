//! The desktop widget layer (design/20 §1.14; macOS Sonoma's desktop widgets): the widgets of
//! this output's own list in the state file, else `widgets.desktop_widgets`, placed on a grid
//! over the wallpaper and under every window, shown while `widgets.desktop` is `On`. One surface
//! per output ([`layer::layer`]); its input region is the whole output, so a press of the
//! secondary button on the wallpaper opens the desktop's menu ([`menu`]), and its blur region is
//! the widgets' frames. Each output's list and where each widget sits are saved state
//! (`$XDG_STATE_HOME/sill/desktop-widgets.json`, [`sill_ui_kit::links::PlacementFile`]), the
//! places set by [`placement::place`]; a widget is moved by dragging it ([`drag`], [`arrange`],
//! [`placed`], [`glide`]), plain pointer events on the layer, at the frame level: nothing inside
//! a widget's own view takes part. A widget taken away plays its exit before it goes
//! ([`leaving`]).

mod arrange;
mod drag;
mod geometry;
mod glide;
mod io;
mod layer;
mod leaving;
mod menu;
mod model;
mod placed;
mod placement;
mod surface;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use placement::{GridSize, grid_size, place};
pub use surface::DesktopWidgets;
pub use view::DESKTOP_WIDGETS_CSS;
