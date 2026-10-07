//! Each output's work area: the output less the bar's and the dock's exclusive zones, with the
//! output's scale. What the shell publishes on `org.quire.Outputs1` so an app can size and
//! place a window against the usable screen, as `NSScreen.visibleFrame` lets a Mac app do.
//! Pure; `sill-services` serves it and the daemon works it out from the surfaces' layer
//! configurations.

mod command;
mod model;
mod reduce;
#[cfg(test)]
mod tests;

pub use command::WorkAreaCommand;
pub use model::{Edge, Rect, WorkArea, WorkAreaLink, WorkAreas, WorkTrouble, Zone};
pub use reduce::{work_area, work_rect};
