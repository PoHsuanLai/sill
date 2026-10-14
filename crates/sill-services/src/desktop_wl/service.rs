//! The compositor's toplevels and workspaces as a [`Service`], on a thread of its own.

use sill_model::windows::{WindowModel, WlCommand};

use super::publish::Publish;
use super::thread;
use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The compositor's toplevels and workspaces as a [`Service`], on a thread of its own (started
/// by `run`) with its own Wayland connection to `env`'s compositor: the compositor's globals
/// pick the arm that follows it, and publish after every dispatch.
#[derive(Debug)]
pub enum WindowsService {}

impl Service for WindowsService {
    const NAME: &'static str = "windows";
    type State = WindowModel;
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
            .name("windows".to_owned())
            .spawn(move || thread::run(route, Publish::new(state), commands));
        std::future::ready(started.map(drop).map_err(ServiceError::Thread))
    }
}
