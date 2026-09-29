//! Hot corners (design/20 §1.16; macOS's Hot Corners): a transparent square of
//! `hot_corners.size_px` in each corner whose action is not `None`, on every output. The pointer
//! resting in one for `hot_corners.dwell_ms` fires it once; it fires again only after the
//! pointer has left and `rearm_ms` has passed ([`step::step`]). A firing is written to the
//! [`crate::links::CornerLink`]; the daemon carries out the action (`crates/sill/src/corners.rs`), since
//! opening the launcher, the notification center or the control center, locking, minimising
//! windows and running a command are its links and services, not a surface's.
//!
//! Never painted: the root draws one transparent element that takes the pointer, no `Ds`, no
//! material, no blur; the compositor still delivers pointer enter and leave to it.

mod io;
mod layer;
mod model;
mod step;
mod surface;
mod view;

pub use model::{active_corners, corner_action, corner_command};
pub use surface::HotCorners;
pub use view::HOT_CORNERS_CSS;
