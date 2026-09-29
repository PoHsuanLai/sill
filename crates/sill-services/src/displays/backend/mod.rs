//! The displays' effects: sysfs, the backlight over logind, `cosmic-randr`, the DDC worker, the
//! record of seen displays and the fontconfig snippet.

mod backlight;
mod ddc;
mod fontconfig_file;
mod run;
pub(crate) mod scale_pass;
mod seen_file;
pub(crate) mod sink;

pub(crate) use run::{Paths, run};
pub use sink::{CosmicRandr, ScaleSink};
