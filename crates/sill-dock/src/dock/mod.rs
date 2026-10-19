//! The dock (design/20 §1.2, design/10, 06 §20.1): one per output, pinned apps then running
//! ones grouped by `match_app`, Plank magnification, labels, badges, clicks, drags and
//! auto-hide, driven by the pure [`machine`].

mod activate;
pub mod context_menu;
mod effects;
mod frame;
mod geometry;
mod hit;
mod icon;
mod io;
mod items;
mod layer;
pub mod machine;
mod menu;
mod menu_geometry;
mod menu_popup;
mod pick;
mod pointer;
mod rest_rects;
mod surface;
mod tile;
mod view;

pub use surface::Dock;
pub use view::DOCK_CSS;
