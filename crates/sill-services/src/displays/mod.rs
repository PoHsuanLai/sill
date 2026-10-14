//! Displays: Mac-quality defaults derived from what each display reports, nothing
//! vendor-specific, every default overridable in `display.*`.
//!
//! - **Scale.** Every output's EDID (size, native mode) gives its density; `default_scale`
//!   aims it at 109 points per inch, and the first time sill sees a display it sets that scale
//!   through the compositor's own output management
//!   ([`OutputsRoute`], picked from the compositor's globals), then never again
//!   (`decide_scale`, `$XDG_STATE_HOME/sill/displays.json`).
//! - **Gamut.** The EDID primaries classified as sRGB, Display P3 or wide (`classify`); stored
//!   for colour management, which no compositor offers yet.
//! - **DDC/CI.** External monitors' brightness, contrast, speaker volume, mute, input and RGB
//!   gains (`Controls`) on one worker thread, with debounced writes and cached reads.
//! - **Brightness routing.** Built-in panels through logind, monitors through DDC
//!   (`BrightnessTarget`), on the output under the pointer or all of them.
//! - **Monitor speakers** follow the software volume when the default sink is the monitor.
//! - **Night warmth** through the monitors' RGB gains (`warmth_gains`), off by default.
//! - **Font rendering** for other apps: a fontconfig snippet from the densest display.
//!
//! The service is inert until the daemon sends its first [`DisplaysCommand::Policy`].

mod backend;

pub use backend::{OutputsRoute, ScaleSink, WlrOutputs};

use std::sync::Arc;

use sill_model::displays::{Displays, DisplaysCommand};
use sill_model::volume::Volume;
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The displays as a [`Service`], following the volume service (its `Deps`) for the
/// monitor-speaker path and talking DDC on `env`'s buses. Until the first `Policy` command it does
/// nothing at all: no bus probe, no output query, no file.
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
        let paths = backend::Paths::of(
            env.buses.system.clone(),
            &env.dirs,
            Arc::clone(&env.ddc),
            env.wayland.clone(),
        );
        async move {
            backend::run(io.state, io.commands, deps, paths).await;
            Ok(())
        }
    }
}
