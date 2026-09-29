//! The one shape every service has: a published state, a command channel, the other services it
//! reads or drives, and a task started against [`Env`]'s seams.
//!
//! A service is a marker type implementing [`Service`]; [`spawn`] starts it and hands back its
//! [`Handle`]. The pure half of a service is its `step` function (one per state machine, always
//! that name, a free function beside the state it steps); `run` is the effectful half that feeds
//! it and publishes.

use std::fmt;
use std::future::Future;

use tokio::sync::{mpsc, watch};

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;

/// One service: what it publishes, what it can be told, what it needs from other services, and
/// the task that does the work.
pub trait Service: Sized + 'static {
    /// The name its log lines carry.
    const NAME: &'static str;
    /// What it publishes; its default means "absent" or "nothing yet".
    type State: Clone + Default + PartialEq + Send + Sync + 'static;
    /// What it can be told.
    type Command: Send + 'static;
    /// Other services' receivers and controls it reads or drives; `()` when none.
    type Deps: Send + 'static;

    /// The value published before `run` starts (the default, unless the service can know better
    /// at once, as the clock and the dock pins can).
    fn initial(env: &Env, deps: &Self::Deps) -> Self::State {
        let _ = (env, deps);
        Self::State::default()
    }

    /// The service's task, spawned on the runtime by [`spawn`]. It takes from `env` what it
    /// reaches outside the process before it returns the future. A service on its own thread
    /// starts that thread from here. An error ends the service with one log line (a bus that is
    /// absent by choice ends it silently); either way its receivers keep the last value.
    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static;
}

/// The service's own halves of its channels.
pub struct ServiceIo<S: Service> {
    /// Where it publishes.
    pub state: watch::Sender<S::State>,
    /// What it is told; `None` once every [`Control`] has gone.
    pub commands: mpsc::UnboundedReceiver<S::Command>,
}

/// A running service as its readers hold it. Cheap to clone.
pub struct Handle<S: Service> {
    pub state: watch::Receiver<S::State>,
    pub control: Control<S::Command>,
}

impl<S: Service> fmt::Debug for ServiceIo<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceIo")
            .field("service", &S::NAME)
            .finish()
    }
}

impl<S: Service> Clone for Handle<S> {
    fn clone(&self) -> Self {
        Handle {
            state: self.state.clone(),
            control: self.control.clone(),
        }
    }
}

impl<S: Service> fmt::Debug for Handle<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle").field("service", &S::NAME).finish()
    }
}

impl<S: Service> Handle<S> {
    /// A handle starting at `state`, and the service's halves of its channels: what [`spawn`]
    /// gives a service's task, and what a test or a demo feeds and reads in its place.
    pub fn channel(state: S::State) -> (Handle<S>, ServiceIo<S>) {
        let (tx, rx) = watch::channel(state);
        let (control, commands) = Control::channel();
        (
            Handle { state: rx, control },
            ServiceIo {
                state: tx,
                commands,
            },
        )
    }
}

/// Start `S` on `rt` against `env`'s seams, reading or driving `deps`.
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S> {
    let (handle, io) = Handle::<S>::channel(S::initial(env, &deps));
    let task = S::run(env, deps, io);
    rt.spawn(async move { report(S::NAME, task.await) });
    handle
}

/// Log why a service ended early; a bus absent by choice is not worth a line.
fn report(name: &str, ended: Result<(), ServiceError>) {
    match ended {
        Ok(()) | Err(ServiceError::NoBus(_)) => {}
        Err(error) => eprintln!("sill: {name}: {error}; it keeps its last value"),
    }
}
