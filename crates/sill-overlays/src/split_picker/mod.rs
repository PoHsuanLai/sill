//! The Split View picker (what the Mac shows when a window has taken one half of the screen):
//! one Overlay surface per output, kept warm and hidden, that appears when the compositor offers
//! the other half ([`sill_ui_kit::links::SplitPickerLink`]). The offer is a dimmed backdrop over
//! the screen, with the first window's half left as it is and, in the other half, quire's
//! `AppSwitcher` with one tile per window that can fill it (the window's app icon, its title
//! under the selected tile). A click on a tile or Return picks it; Left and Right move the
//! selection; Escape, or a click anywhere else, turns the choice down. Keyboard `Exclusive`.
//!
//! The answer is written back to the link and the daemon sends it to the compositor, which ends
//! the offer (the daemon then closes it on the link); the surface hides itself at the answer so
//! it does not wait for that round trip.

mod io;
mod model;
mod surface;
mod view;

pub use surface::SplitPicker;
pub use view::SPLIT_PICKER_CSS;
