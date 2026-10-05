//! `sill session import-input --from-plasma`: carry the per-device mouse and touchpad settings
//! Plasma keeps in `kcminputrc` over to cosmic-comp's `input_devices` file.
//!
//! cosmic-comp keys those settings by the device's name, and the names are the person's own
//! hardware, so no seed in `dist/` can hold them; this step reads them where they are. The parts:
//! [`wire`] (cosmic-comp's file, as data), [`plasma`] (the pure translation, text to settings and
//! settings to RON) and [`run`] (reading `kcminputrc`, writing the file, the report).

mod plasma;
mod run;
pub mod wire;

pub use plasma::{import, render};
pub use run::{Args, main};
