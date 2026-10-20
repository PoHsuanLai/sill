//! sill serves each display's brightness on the session bus, so Settings can show a slider
//! per display and follow the keys and the control centre: `org.quire.Displays1` at
//! `/org/quire/Displays1`, a small interface over what the displays service already routes
//! (logind for built-in panels, DDC/CI for monitors).
//!
//! - `Brightness() -> a(su)`: every output whose brightness can be read, with its percentage
//!   (0 to 100); an output with nothing to read is not listed.
//! - `SetBrightness(s output, u percent)`: sets that output's own brightness. Only Settings may
//!   call it (the program the table of the desktop's own programs names as `settings`, owning
//!   `org.quire.Settings`, [`crate::callers`]); anyone else is refused (`AccessDenied`). An
//!   output that is not listed, or a percentage above 100, is `InvalidArgs`.
//! - `Changed(a(su))`: the same list, sent whenever it changes (a key, the control centre, a
//!   monitor adjusted on itself while a view follows).
//! - **The name.** `org.quire.Displays1`, claimed at start unless [`Env::claims`](crate::env) is
//!   `Passive` (a test's `start` never claims it). Reading is open to any program.

mod object;
mod run;

use sill_model::displays::{Displays, DisplaysCommand};
use sill_model::serve::ServeLink;
use std::convert::Infallible;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The well-known name the interface is served under.
pub const BUS: &str = "org.quire.Displays1";
/// Where the object is exported.
pub const PATH: &str = "/org/quire/Displays1";

/// What the service reads and drives: the displays it lists and the displays service it tells.
#[derive(Debug, Clone)]
pub struct DisplaysBusLinks {
    pub displays: watch::Receiver<Displays>,
    pub control: Control<DisplaysCommand>,
}

/// The displays' bus interface as a [`Service`]; its state says whether the name is held.
#[derive(Debug)]
pub enum DisplaysBusService {}

impl Service for DisplaysBusService {
    const NAME: &'static str = "displays-bus";
    type State = ServeLink;
    type Command = Infallible;
    type Deps = DisplaysBusLinks;

    fn run(
        env: &Env,
        deps: DisplaysBusLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let claims = env.claims;
        let callers = std::sync::Arc::clone(&env.callers);
        async move {
            run::run(bus, claims, callers, deps, io).await;
            Ok(())
        }
    }
}
