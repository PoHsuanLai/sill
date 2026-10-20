//! The permission sheet: a yes-or-no question about an app, asked of the person the way macOS asks
//! it. transom (the portal backend) calls `org.quire.Shell1.AskPermission`
//! (`sill_services::capture_pick`) before it lets an app control the desktop or take a screenshot
//! the person did not start; this sheet is quire's `Alert` centred on the sheets' output over an
//! Overlay layer that takes the keyboard. It shows the app's icon, a headline, the app's name with
//! its explanation, and two buttons labelled as the question says: the yes button is the default
//! (Return), the no button answers Escape. The answer goes back through
//! `CaptureCommand::PermissionAnswer`.
//!
//! The words are an app's, so the model has cleaned and cut them (`PermissionAsk::from_wire`)
//! before they reach this surface.
//!
//! The compositor asks the same way when a program wants a power (the screen, the keys, the
//! clipboard, ...): its question is the same sheet, in the same line as the portal's, one at a
//! time (`sill_model::capture_pick::{consent_ask, consent_words}` make its words). Its buttons are
//! Allow (Allow Once for a program the desktop cannot recognise) and Don't Allow; Escape is a
//! dismissal that nothing keeps, not the no button.
//!
//! No question takes an answer for [`PERMISSION_ARM_AFTER`] after it is drawn: the buttons are
//! disabled and Return, Escape and clicks do nothing until then (quire's `Alert` `arm_after`).

mod arming;
mod io;
mod surface;
mod view;
mod words;

pub use arming::PERMISSION_ARM_AFTER;
pub use surface::PermissionSheet;
