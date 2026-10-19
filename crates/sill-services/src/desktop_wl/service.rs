//! The compositor's toplevels and workspaces as a [`Service`], on a thread of its own.

use sill_model::windows::{WindowModel, WlCommand};

use tokio::sync::oneshot;

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
        let (done, ended) = oneshot::channel();
        let started = std::thread::Builder::new()
            .name("windows".to_owned())
            .spawn(move || {
                thread::run(route, Publish::new(state), commands);
                let _waited_for_or_not = done.send(());
            });
        // The run lasts as long as the thread: a run that ended at once would be taken for a
        // service with nothing to do, its commands dropped and the thread stopped, so the
        // windows stayed empty for good (casement, 2026-10-20).
        async move {
            started.map_err(ServiceError::Thread)?;
            let _thread_ended_or_dropped_its_sender = ended.await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_testkit::TempDir;

    use super::WindowsService;
    use crate::env::Env;
    use crate::service::{Handle, Service};

    /// The run is not over while the thread follows (here: waits on its commands with no
    /// compositor to reach), and is over once every control is gone.
    #[tokio::test]
    async fn the_run_lasts_as_long_as_its_thread() {
        let scratch = TempDir::new("windows-run");
        let env = Env::isolated(scratch.path());
        let (handle, io) = Handle::<WindowsService>::channel(Default::default());
        let mut run = tokio::spawn(WindowsService::run(&env, (), io));
        let early = tokio::time::timeout(Duration::from_millis(200), &mut run).await;
        assert!(early.is_err(), "the run ended while its thread still runs");
        drop(handle);
        let ended = tokio::time::timeout(Duration::from_secs(5), run).await;
        assert!(
            matches!(ended, Ok(Ok(Ok(())))),
            "the run ends with its thread: {ended:?}"
        );
    }
}
