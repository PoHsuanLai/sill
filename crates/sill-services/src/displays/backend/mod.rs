//! The displays' effects: sysfs, the backlight over logind, `cosmic-randr` or the native output
//! management (picked from the compositor's globals), the DDC worker, the
//! record of seen displays and the fontconfig snippet.

mod backlight;
mod ddc;
mod fontconfig_file;
mod run;
pub(crate) mod scale_pass;
mod seen_file;
pub(crate) mod sink;
mod sink_wlr;

pub(crate) use run::{Paths, run};
pub use sink::{CosmicRandr, ScaleSink};
pub use sink_wlr::{OutputsRoute, WlrOutputs};
