//! DDC/CI: external monitors' own controls (brightness, contrast, speaker volume, mute, input,
//! RGB gain) over the display cable's i2c lines. One worker thread owns every bus
//! ([`worker::run`]); writes are debounced ([`debounce`]); reads are cached in what the worker
//! publishes.

pub mod bus;
pub mod debounce;
#[cfg(test)]
pub(crate) mod fake;
pub mod i2c;
pub mod matching;
pub mod retry;
pub mod vcp;
pub mod worker;

pub use bus::{DdcBus, DdcError, DdcId, Endpoint};
pub use debounce::{QUIET, Write};
pub use i2c::I2cBuses;
pub use vcp::{ColorPreset, Controls, Feature, InputSource, Level, RgbGain, mute_value};
pub use worker::{DdcDisplay, DdcJob};
