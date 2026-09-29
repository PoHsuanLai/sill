//! DDC/CI: external monitors' own controls (brightness, contrast, speaker volume, mute, input,
//! RGB gain) over the display cable's i2c lines: the VCP features and levels, which bus is which
//! display, and the debounce that settles writes. The worker thread that owns every bus is the
//! service's backend.

pub mod debounce;
mod display;
pub mod matching;
pub mod vcp;

pub use debounce::{QUIET, Write};
pub use display::{DdcDisplay, Endpoint};
pub use vcp::{ColorPreset, Controls, Feature, InputSource, Level, RgbGain, mute_value};
