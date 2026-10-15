//! Night Shift: casement's software night light, asked for by the person. casement warms the
//! screen from `[night_light]` in `~/.config/casement/settings.toml`; this service reads that
//! table and writes it when the person turns Night Shift on until tomorrow, turns it off or
//! moves the warmth, and keeps the manual turn-on's end in `night-shift.json` in the state
//! directory so a restart does not leave the screen warm all day. Everything else in casement's
//! file is left as it is (the table is merged key by key); an edit made elsewhere (Settings) is
//! read back through a directory watch.
//!
//! Where sill warms the displays itself (`display.night_warmth`), the shell tells this service
//! (`NightShiftCommand::Backend`) and casement is kept off.

mod backend;

use sill_model::night_shift::{NightShift, NightShiftCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Night Shift as a [`Service`].
#[derive(Debug)]
pub enum NightShiftService {}

impl Service for NightShiftService {
    const NAME: &'static str = "night-shift";
    type State = NightShift;
    type Command = NightShiftCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let casement = env.dirs.config.join("casement");
        let state = env.dirs.sill_state();
        let clock = std::sync::Arc::clone(&env.clock);
        async move {
            backend::run(casement, state, clock, io).await;
            Ok(())
        }
    }
}
