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
pub use ddc::{Controls, DdcDisplay, Feature, Level};
pub use drm::Placement;
pub use edid::{Edid, Millimetres, Mode, Pixels, Primaries};
pub use font::{FontRendering, Hinting, font_rendering_for};
pub use gamut::{Gamut, classify};
pub use identity::IdentityKey;
pub use model::{Displays, DisplaysCommand, DisplaysEvent, FOLLOW_EVERY_MS, Following, step};
pub use output::DisplayOutput;
pub use policy::{
    DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmth, ScaleMode,
    ScaleOverride,
};
pub use scale::{DisplayScale, Ppi, ScaleRounding, default_scale};
pub use seen::decide_scale;
pub use sink::{CosmicRandr, ScaleSink};
pub use warmth::warmth_gains;

use std::sync::Arc;

use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};
use sill_model::volume::Volume;

/// The displays as a [`Service`], following the volume service (its `Deps`) for the
/// monitor-speaker path and talking DDC on `env`'s buses. Until the first `Policy` command it does
/// nothing at all: no bus probe, no `cosmic-randr`, no file.
#[derive(Debug)]
pub enum DisplaysService {}

impl Service for DisplaysService {
    const NAME: &'static str = "displays";
    type State = Displays;
    type Command = DisplaysCommand;
    type Deps = watch::Receiver<Volume>;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let paths = run::Paths::of(env.buses.system.clone(), &env.dirs, Arc::clone(&env.ddc));
        async move {
            run::run(io.state, io.commands, deps, paths).await;
            Ok(())
        }
    }
}
