//! COSMIC's toplevel and workspace state, on a thread with its own Wayland connection and cctk
//! states.
//!
//! The sctk/cctk delegate macros live in `state.rs` only, the handler impls in `handlers.rs`,
//! so an sctk major bump touches those two files. On a compositor without the COSMIC protocols
//! the state stays empty, the thread parks on the command channel and commands are
//! dropped: the dock degrades quietly (design/10 §10.7).

mod backend;

pub(crate) use backend::{Publish, forward_commands, park_commands};
use sill_model::cosmic_wl::{CosmicWl, WlCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// COSMIC's toplevels and workspaces as a [`Service`], on a thread of its own (started by `run`)
/// with its own Wayland connection to `env`'s compositor: it binds the cctk globals and
/// publishes after every `done`.
#[derive(Debug)]
pub enum CosmicWlService {}

impl Service for CosmicWlService {
    const NAME: &'static str = "cosmic_wl";
    type State = CosmicWl;
    type Command = WlCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let route = env.wayland.clone();
        let ServiceIo { state, commands } = io;
        let started = std::thread::Builder::new()
            .name("cosmic_wl".to_owned())
            .spawn(move || backend::run(route, Publish::new(state), commands));
        std::future::ready(started.map(drop).map_err(ServiceError::Thread))
    }
}
