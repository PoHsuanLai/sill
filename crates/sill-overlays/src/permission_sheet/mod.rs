//! The permission sheet: a yes-or-no question about an app, asked of the person the way macOS asks
//! it. transom (the portal backend) calls `org.quire.Shell1.AskPermission`
//! (`sill_services::capture_pick`) before it lets an app control the desktop or take a screenshot
//! the person did not start; this sheet is quire's `Alert` centred on the banners' output over an
//! Overlay layer that takes the keyboard. It shows the app's icon, a headline, the app's name with
//! its explanation, and two buttons labelled as the question says: the yes button is the default
//! (Return), the no button answers Escape. The answer goes back through
//! `CaptureCommand::PermissionAnswer`.
//!
//! The words are an app's, so the model has cleaned and cut them (`PermissionAsk::from_wire`)
//! before they reach this surface.

mod io;
mod layer;
mod surface;
mod view;
mod words;

pub use surface::PermissionSheet;
