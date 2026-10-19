//! The displays' effects: sysfs, the backlight over logind, the native output
//! management (picked from the compositor's globals, read only), the DDC worker and the
//! fontconfig snippet.

mod backlight;
mod ddc;
mod fontconfig_file;
mod outputs_route;
mod run;

pub use outputs_route::{OutputsRoute, WlrOutputs};
pub(crate) use run::{Paths, run};
