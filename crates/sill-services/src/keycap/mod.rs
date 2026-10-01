//! keycap, the Mac keyboard daemon: sill tells it which profile the focused window wants and
//! the person's `keycap.toml`, learns from it whether it runs and holds the keyboards, and tells
//! the person when that needs them (`sill_model::keycap::attention`).
//!
//! The socket (`/run/keycap/keycap.sock`, `Env::keycap_socket`) is held by a thread of its own,
//! started by `run`: it reconnects with a growing wait and starts each connection with the focus
//! and the config. While the socket is down for more than a few seconds the service asks systemd
//! about the unit on the system bus, so a crash loop that gave up shows as `Failed`; a machine
//! with no keycap shows as `NotInstalled`. `Retry` resets and restarts the unit, `Resume` ends the
//! pause the escape chord caused.
//!
//! Nothing is sent without a socket path (`Env::isolated` has none) and no unit is asked about
//! without a system bus, so a test or a script never touches the real daemon.
//!
//! `Overrides` (the person's `keyboard.overrides`) replaces the service's own and re-chooses the
//! focused window's profile, sending `Focus` when it changed.
//!
//! The notices carry no action: an action of sill's own notification reaches no service, so
//! recovery is `sill keycap retry` and `sill keycap resume`.

mod backend;

use sill_model::keycap::{KeycapCommand, KeycapState};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::KeycapLinks;

/// The keycap connection as a [`Service`], on `env`'s socket and system bus, reading the focused
/// window and posting its notices through the services in its [`KeycapLinks`].
#[derive(Debug)]
pub enum KeycapService {}

impl Service for KeycapService {
    const NAME: &'static str = "keycap";
    type State = KeycapState;
    type Command = KeycapCommand;
    type Deps = KeycapLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move { backend::run(reach, deps, io).await }
    }
}
