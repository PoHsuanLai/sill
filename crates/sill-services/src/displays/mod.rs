//! Displays: Mac-quality defaults derived from what each display reports, nothing
//! vendor-specific, every default overridable in `display.*`.
//!
//! - **Scale.** Every output's EDID (size, native mode) gives its density; [`default_scale`]
//!   aims it at 109 points per inch, and the first time sill sees a display it sets that scale
//!   through `cosmic-randr` ([`ScaleSink`]), then never again ([`decide_scale`],
//!   `$XDG_STATE_HOME/sill/displays.json`).
//! - **Gamut.** The EDID primaries classified as sRGB, Display P3 or wide ([`classify`]); stored
//!   for colour management, which cosmic-comp 1.8 does not offer yet.
//! - **DDC/CI.** External monitors' brightness, contrast, speaker volume, mute, input and RGB
//!   gains ([`Controls`]) on one worker thread, with debounced writes and cached reads.
//! - **Brightness routing.** Built-in panels through logind, monitors through DDC
//!   ([`BrightnessTarget`]), on the output under the pointer or all of them.
//! - **Monitor speakers** follow the software volume when the default sink is the monitor.
//! - **Night warmth** through the monitors' RGB gains ([`warmth_gains`]), off by default.
//! - **Font rendering** for other apps: a fontconfig snippet from the densest display.
//!
//! The service is inert until the daemon sends its first [`DisplaysCommand::Policy`].

pub mod backlight;
pub mod brightness;
pub mod ddc;
pub mod drm;
pub mod edid;
pub mod font;
pub mod fontconfig_file;
pub mod gamut;
pub mod hw_volume;
pub mod identity;
pub mod model;
pub mod output;
pub mod plan;
pub mod policy;
pub mod randr;
pub mod run;
pub mod scale;
pub mod scale_pass;
pub mod seen;
pub mod sink;
pub mod warmth;

pub use brightness::{
    Backlight, BacklightKind, BacklightName, BrightnessTarget, LevelChange, stepped,
};
pub use ddc::{Controls, DdcBus, DdcDisplay, DdcError, DdcId, Feature, I2cBuses, Level};
pub use drm::Placement;
pub use edid::{Edid, Millimetres, Mode, Pixels, Primaries};
pub use font::{FontRendering, Hinting, font_rendering_for};
pub use gamut::{Gamut, classify};
pub use identity::IdentityKey;
pub use model::{
    Displays, DisplaysCommand, DisplaysEvent, FOLLOW_EVERY_MS, Following, reduce_displays,
};
pub use output::DisplayOutput;
pub use policy::{
    DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth, ScaleMode,
    ScaleOverride,
};
pub use scale::{DisplayScale, Ppi, ScaleRounding, default_scale};
pub use seen::decide_scale;
pub use sink::{CosmicRandr, ScaleSink};
pub use warmth::warmth_gains;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::{Bus, Dirs};
use crate::volume::Volume;

/// Start the displays task on `rt`, following `volume` for the monitor-speaker path. Until the
/// first `Policy` command it does nothing at all: no bus probe, no `cosmic-randr`, no file.
pub fn spawn(
    rt: &Handle,
    system: Bus,
    dirs: &Dirs,
    volume: watch::Receiver<Volume>,
) -> (watch::Receiver<Displays>, Control<DisplaysCommand>) {
    let (tx, rx) = watch::channel(Displays::default());
    let (control, commands) = Control::channel();
    rt.spawn(run::run(tx, commands, volume, run::Paths::of(system, dirs)));
    (rx, control)
}
