//! The idle dim: before the displays go off for idleness, every output gets a black scrim at
//! `100 - idle.dim_level_pct` percent, quire's `IdleDim`. It is an overlay, never a brightness
//! change, so a crash while dimmed leaves nothing dark.
//!
//! It follows the idle service's published `Idle.dim` alone: `Dimmed(level)` fades the scrim in
//! over `--t-idle-dim`, `Clear` (any input, the displays going off, an inhibitor) snaps it out.
//! The surface (`layer.rs`) is on the Overlay layer, so the session-lock surfaces, which
//! the compositor always stacks above every layer surface, cover it; its input region is empty
//! and it takes no keyboard, so it never counts as activity or steals a click. The daemon maps
//! it only while dimmed ([`visibility_of`]), so an awake output draws nothing at all.

mod io;
mod layer;
mod model;
mod surface;
mod view;

pub use model::visibility_of;
pub use surface::IdleDim;
