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
use crate::supervise::panic_text;

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

/// Start `S` on `rt` against `env`'s seams, reading or driving `deps`. The service's task is
/// watched by a second one, so a panic inside it is caught at this boundary and logged with the
/// service's name instead of vanishing with the task.
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S> {
    let (handle, io) = Handle::<S>::channel(S::initial(env, &deps));
    let work = rt.spawn(S::run(env, deps, io));
    rt.spawn(async move {
        if let Some(line) = ending_line(S::NAME, work.await) {
            eprintln!("{line}");
        }
    });
    handle
}

/// The log line for how a service's task ended, or none when that is not worth one: a clean end,
/// a bus absent by choice, or the runtime shutting down.
fn ending_line(
    name: &str,
    ended: Result<Result<(), ServiceError>, tokio::task::JoinError>,
) -> Option<String> {
    match ended {
        Ok(Ok(())) | Ok(Err(ServiceError::NoBus(_))) => None,
        Ok(Err(error)) => Some(format!("sill: {name}: {error}; it keeps its last value")),
        Err(error) => error.try_into_panic().ok().map(|payload| {
            format!(
                "sill: {name}: panicked: {}; it keeps its last value",
                panic_text(payload.as_ref())
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::ending_line;
    use crate::env::BusKind;
    use crate::error::ServiceError;

    fn broken() -> Result<(), ServiceError> {
        panic!("the thing broke")
    }

    #[tokio::test]
    async fn how_a_service_ended_is_logged_only_when_it_matters() {
        let panicked = tokio::spawn(async { broken() }).await;
        let cases = [
            ("a clean end", Ok(Ok(())), None),
            (
                "no bus by choice",
                Ok(Err(ServiceError::NoBus(BusKind::System))),
                None,
            ),
            (
                "an error",
                Ok(Err(ServiceError::Gone)),
                Some("sill: svc: the service has stopped; it keeps its last value"),
            ),
            (
                "a panic",
                panicked,
                Some("sill: svc: panicked: the thing broke; it keeps its last value"),
            ),
        ];
        for (name, ended, want) in cases {
            assert_eq!(ending_line("svc", ended).as_deref(), want, "{name}");
        }
    }
}
