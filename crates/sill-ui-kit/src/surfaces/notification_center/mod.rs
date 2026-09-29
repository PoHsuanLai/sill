//! The notification center (design/20 §1.6): the history grouped by app, with Clear, and a slot
//! for widgets, in a column that slides in from the output's right edge under the bar.
//!
//! One Overlay surface per output, spawned hidden and kept warm like the power menu, following
//! one shared [`crate::links::CenterLink`]. Opened by a click on the bar's clock (as macOS 11 on) and by
//! `sill notifications {toggle,open}`; closed by the clock again, Escape, a click outside the
//! column, or `sill notifications close`. Why a layer and not a popup: `layer.rs`.

pub mod groups;
pub mod keymap;
pub mod keys;
pub mod labels;
pub mod phase;
pub mod rows;
pub mod widgets;

mod column;
mod contents;
mod drawn;
mod group;
mod io;
mod layer;
mod surface;
mod view;

pub use groups::{Expanded, GroupView, group_views, toggle_group};
pub use keys::{CenterKey, CenterOut, RowFocus, step_keys};
pub use layer::{PANEL_ID, PANEL_PADDING, PANEL_RADIUS};
pub use surface::NotificationCenter;
pub use view::NOTIFICATION_CENTER_CSS;
pub use widgets::CenterWidgets;
