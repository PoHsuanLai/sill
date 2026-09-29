//! The desktop widget layer (design/20 §1.14; macOS Sonoma's desktop widgets): the
//! widgets `widgets.desktop_widgets` names, placed on a grid over the wallpaper and under every
//! window, shown while `widgets.desktop` is `On`. One surface per output
//! ([`layer::layer`]); its input and blur regions are the widgets' frames only, so a
//! click between them reaches the desktop below. Where each widget sits is saved state, per
//! output (`$XDG_STATE_HOME/sill/desktop-widgets.json`, [`crate::links::PlacementFile`]), placed by
//! [`placement::place`]; a widget is moved by dragging it ([`drag`], [`arrange`], [`placed`],
//! [`glide`]), plain pointer events on the layer, at the frame level:
//! nothing inside a widget's own view takes part. A widget taken away plays its exit before it
//! goes ([`leaving`]).

mod arrange;
mod drag;
mod geometry;
mod glide;
mod io;
mod layer;
mod leaving;
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
