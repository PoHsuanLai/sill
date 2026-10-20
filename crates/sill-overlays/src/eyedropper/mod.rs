//! The colour picker's eyedropper (what an app gets when it asks the person for a colour from the
//! screen, `org.quire.Shell1.PickColor`): one Overlay surface per output, kept warm and hidden,
//! that maps over a still picture of its output while a colour is asked for
//! ([`sill_ui_kit::links::EyedropperLink`]). A round magnifier follows the pointer and shows the
//! pixels around it enlarged, the one under the pointer ringed in the middle. A click picks that
//! pixel's colour; Escape or the other button cancels. Nothing else is shown, as on the Mac: no
//! readout of the value. Keyboard `Exclusive`.
//!
//! The surface says it exists (the link counts it) so the capture service can tell an app a
//! colour can be asked for; the answer goes to the capture service, which ends the question, and
//! the surface hides at the answer without waiting for that round trip.

mod io;
mod model;
mod surface;
mod view;

pub use surface::ColorSampler;
pub use view::EYEDROPPER_CSS;
