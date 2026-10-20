//! The bar (design/20 §1.1, design/13 §13.3.1-13.3.4, 06 §20.3): one per output, left the app
//! name and the workspace indicator, right the tray, the control center modules shown as their
//! own items (`control_center.menu_bar_*`: Sound, Wi-Fi and Battery by default), the control
//! center's item and the clock; every menu an `xdg_popup`, tracked by [`track`].

mod app_name;
mod app_titles;
mod clock;
mod content;
mod control_center_item;
mod dbusmenu;
mod io;
mod layer;
mod menu_popup;
mod menus;
mod model;
mod module_item;
mod pick;
mod plan;
mod popup;
mod reorder;
mod status_item;
mod surface;
mod system_item;
mod title;
pub mod track;
mod tray;
mod tray_input;
mod view;
mod workspaces;

pub(crate) use menus::BarMenu;
pub(crate) use status_item::StatusIcon;
pub use surface::Bar;
pub use view::BAR_CSS;
