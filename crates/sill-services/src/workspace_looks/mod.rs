//! The workspaces and their looks on the session bus: `org.quire.Workspaces1`, the shell's answer
//! to "which workspace is this window on, and how is it tinted" for an app that keeps its Spaces
//! from the desktop.
//!
//! - **Interface `org.quire.Workspaces1`** at `/org/quire/Workspaces1`.
//!   - Property `Workspaces`, type `a(ssssb)`: per workspace its id (the compositor's stable id,
//!     else the shell's key for the session), name, output (the connector name of its group's first
//!     output, empty when none), look (a `SpaceLook` as quire's compact JSON, at most 1024 bytes)
//!     and whether it is the active workspace of its group.
//!   - Change signal: the standard `PropertiesChanged` for `Workspaces`, sent whenever a
//!     workspace, a name, a look or the active one changes.
//! - Read-only: nothing can be set or called.
//! - **Claims the name at start**, unless [`Env::claims`](crate::env::Env) is `Passive` (a test's
//!   `start` never claims it).

mod backend;

use sill_model::workspace_looks::{WorkspaceLooks, WorkspaceLooksCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The well-known name of the interface and its object path.
pub const BUS_NAME: &str = "org.quire.Workspaces1";
pub const PATH: &str = "/org/quire/Workspaces1";

/// The workspaces and their looks as a [`Service`].
#[derive(Debug)]
pub enum WorkspaceLooksService {}

impl Service for WorkspaceLooksService {
    const NAME: &'static str = "workspace-looks";
    type State = WorkspaceLooks;
    type Command = WorkspaceLooksCommand;
    type Deps = ();

    fn run(
        env: &Env,
        _deps: (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let claims = env.claims;
        async move {
            backend::run(bus, claims, io).await;
            Ok(())
        }
    }
}
