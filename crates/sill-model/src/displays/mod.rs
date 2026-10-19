//! The displays: every output's description, the DDC monitors and backlights, and the
//! pure rules for brightness, warmth, font rendering and the monitor speakers.

pub mod brightness;
mod command;
pub mod ddc;
pub mod drm;
pub mod edid;
pub mod font;
pub mod gamut;
pub mod hw_volume;
pub mod identity;
mod model;
pub mod output;
pub mod plan;
pub mod policy;
pub mod ppi;
pub mod randr;
pub mod scale;
mod step;
#[cfg(test)]
mod tests;
pub mod warmth;

pub use brightness::{
    Backlight, BacklightKind, BacklightName, BrightnessTarget, LevelChange, stepped,
};
pub use command::{DisplaysCommand, DisplaysEvent};
pub use ddc::{Controls, DdcDisplay, Feature, Level};
pub use drm::Placement;
pub use edid::{Edid, Millimetres, Mode, Pixels, Primaries};
pub use font::{FontRendering, Hinting, font_rendering_for};
pub use gamut::{Gamut, classify};
pub use identity::IdentityKey;
pub use model::{Displays, FOLLOW_EVERY_MS, Following};
pub use output::DisplayOutput;
pub use policy::{DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth};
pub use ppi::Ppi;
pub use scale::DisplayScale;
pub use step::step;
pub use warmth::warmth_gains;
