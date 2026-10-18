//! Starting apps (activation token, DBusActivatable via org.freedesktop.Application.Activate,
//! Terminal=true wrapper, `systemd-run --user --scope --collect --unit=app-sill-<id>-<rand>.scope`
//! (fallback process group)).
//!
//! The activation token comes from shell-host (`SurfaceHandle::activation_token`) in the
//! daemon's `sill::launch`, which then sends a [`LaunchCommand`] here. [`LaunchService`] prefers
//! `org.freedesktop.Application.Activate` for a `DBusActivatable` entry (`dbus.rs`); otherwise
//! it plans the argument vector (`plan.rs`, pure) and runs it inside a transient
//! `systemd-run --user --scope`, wrapped in the machine's terminal (`terminal.rs`) when the
//! entry asks for one, tracking the scope until it exits (`run.rs`).

mod backend;
#[cfg(test)]
mod tests;

use backend::Reach;
use sill_model::launch::{LaunchCommand, Launches};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::{LaunchLinks, file_uri, monotonic_now, open_argv, reveal_folder};
pub(crate) use backend::terminal_wrapper;

/// App launching as a [`Service`]: scopes, activation tokens, opening files. Its `Deps` is the
/// app index, which resolves ids to entries, and the notifications service that tells the person
/// an app did not open.
#[derive(Debug)]
pub enum LaunchService {}

impl Service for LaunchService {
    const NAME: &'static str = "launch";
    type State = Launches;
    type Command = LaunchCommand;
    type Deps = LaunchLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = Reach::of(env);
        async move {
            backend::run(reach, deps, io.state, io.commands).await;
            Ok(())
        }
    }
}
