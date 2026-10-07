//! The work areas on the session bus: `org.quire.Outputs1`, the shell's answer to "how much of
//! the screen may my window use, and at what scale".
//!
//! - **Interface `org.quire.Outputs1`** at `/org/quire/Outputs1`.
//!   - Property `WorkAreas`, type `a(us(iiuu)(iiuu)u)`: per output its id, connector name,
//!     frame (x, y, width, height), work area (the frame less the bar's and the dock's exclusive
//!     zones, same order) and scale in 120ths (120 is 1x). Logical pixels.
//!   - Change signal: the standard `PropertiesChanged` for `WorkAreas`, sent whenever an output,
//!     a panel's height or the dock's reveal changes the list.
//! - Read-only: nothing can be set or called.
//! - **Passive until [`WorkAreaCommand::Serve`]**: a test's `start` never claims the name.

mod backend;

use sill_model::work_areas::{WorkAreaCommand, WorkAreas};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The well-known name of the interface and its object path (quire's `ds-desktop` names the
/// same one for its `Outputs` capability).
pub const BUS_NAME: &str = "org.quire.Outputs1";
pub const PATH: &str = "/org/quire/Outputs1";

/// The work areas as a [`Service`].
#[derive(Debug)]
pub enum WorkAreaService {}

impl Service for WorkAreaService {
    const NAME: &'static str = "work-areas";
    type State = WorkAreas;
    type Command = WorkAreaCommand;
    type Deps = ();

    fn run(
        env: &Env,
        _deps: (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move {
            backend::run(bus, io).await;
            Ok(())
        }
    }
}
