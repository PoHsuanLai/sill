//! The one shape every service has: a published state, a command channel, the other services it
//! reads or drives, and a task started against [`Env`]'s seams.
//!
//! A service is a marker type implementing [`Service`]; [`spawn`] starts it and hands back its
//! [`Handle`]. The pure half of a service is its `step` function (one per state machine, always
//! that name, a free function beside the state it steps); `run` is the effectful half that feeds
//! it and publishes.

use std::fmt;
use std::future::Future;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::restart::{Ending, Next, Tries, ending_of};

/// One service: what it publishes, what it can be told, what it needs from other services, and
/// the task that does the work.
pub trait Service: Sized + 'static {
    /// The name its log lines carry.
    const NAME: &'static str;
    /// What it publishes; its default means "absent" or "nothing yet".
    type State: Clone + Default + PartialEq + Send + Sync + 'static;
    /// What it can be told.
    type Command: Send + 'static;
    /// Other services' receivers and controls it reads or drives; `()` when none. It is cloned for
    /// each restart of the service.
    type Deps: Clone + Send + 'static;

    /// The value published before `run` starts (the default, unless the service can know better
    /// at once, as the clock and the dock pins can).
    fn initial(env: &Env, deps: &Self::Deps) -> Self::State {
        let _ = (env, deps);
        Self::State::default()
    }

    /// The service's task, spawned on the runtime by [`spawn`]. It takes from `env` what it
    /// reaches outside the process before it returns the future. A service on its own thread
    /// starts that thread from here. A panic or an error ends the run: [`spawn`] starts it again after
    /// a wait, and leaves it stopped, with a log line, once the tries are used up (a bus that is
    /// absent by choice ends it silently); its receivers then keep the last value.
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

/// How long a run is given to finish once every [`Control`] is gone and its commands have closed:
/// long enough to hand a bus name back and flush a file, short enough that a run that never
/// looks at its commands (it ends some other way) does not hold its keeper.
const WIND_DOWN: Duration = Duration::from_secs(5);

/// Start `S` on `rt` against `env`'s seams, reading or driving `deps`, and keep it running: a
/// task that panics, returns an error, or ends cleanly after running a while, while its
/// [`Control`]s are still held, is started again
/// after a growing wait, up to `RESTARTS_MAX` times in a row (`restart.rs`), and each
/// ending is logged with the service's name. A restarted service starts from the state it last
/// published; commands sent while it was down wait for it.
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S> {
    spawn_with(rt, env, deps, Tries::default())
}

/// [`spawn`] with the given restart waits.
pub(crate) fn spawn_with<S: Service>(
    rt: &tokio::runtime::Handle,
    env: &Env,
    deps: S::Deps,
    tries: Tries,
) -> Handle<S> {
    let (handle, outer) = Handle::<S>::channel(S::initial(env, &deps));
    rt.spawn(keep_running::<S>(env.clone(), deps, outer, tries));
    handle
}

/// Run `S` against `outer`'s channels until its controls are gone or it cannot be run again.
///
/// `S::run` consumes the channel halves it is given, so each run gets a fresh pair and this task
/// carries commands in and state out between that pair and `outer`.
async fn keep_running<S: Service>(
    env: Env,
    deps: S::Deps,
    mut outer: ServiceIo<S>,
    mut tries: Tries,
) {
    loop {
        let started = Instant::now();
        let Some(ending) = run_once::<S>(&env, deps.clone(), &mut outer).await else {
            return;
        };
        match tries.after(S::NAME, &ending, started.elapsed()) {
            Next::Restart { wait, line } => {
                eprintln!("{line}");
                tokio::time::sleep(wait).await;
            }
            Next::Stop { line } => {
                if let Some(line) = line {
                    eprintln!("{line}");
                }
                return;
            }
        }
    }
}

/// One run of `S`, from the state `outer` holds. `None` when every [`Control`] has gone: the run
/// is told so by closing its own commands, and is waited for.
async fn run_once<S: Service>(
    env: &Env,
    deps: S::Deps,
    outer: &mut ServiceIo<S>,
) -> Option<Ending> {
    let (mut inner, io) = Handle::<S>::channel(outer.state.borrow().clone());
    let mut work = tokio::spawn(S::run(env, deps, io));
    loop {
        tokio::select! {
            command = outer.commands.recv() => match command {
                Some(command) => {
                    // The run may have just ended; the command then waits in `outer` no more.
                    let _gone_is_dropped = inner.control.send(command);
                }
                None => {
                    drop(inner.control);
                    if tokio::time::timeout(WIND_DOWN, &mut work).await.is_err() {
                        work.abort();
                    }
                    publish(&outer.state, &inner.state);
                    return None;
                }
            },
            changed = inner.state.changed() => {
                if changed.is_err() {
                    // The run dropped its half: it is ending.
                    let ended = (&mut work).await;
                    publish(&outer.state, &inner.state);
                    return Some(ending_of(ended));
                }
                // Every send the run makes reaches its readers, as it did before restarts: a
                // send of the same value still says "settled" to whoever waits on it.
                outer.state.send_replace(inner.state.borrow().clone());
            }
            ended = &mut work => {
                publish(&outer.state, &inner.state);
                return Some(ending_of(ended));
            }
        }
    }
}

/// Make `to` hold what `from` holds once a run has ended, waking its readers only when that
/// differs (the ending itself is not a send).
fn publish<T: Clone + PartialEq>(to: &watch::Sender<T>, from: &watch::Receiver<T>) {
    let now = from.borrow().clone();
    to.send_if_modified(|held| {
        if *held == now {
            return false;
        }
        *held = now;
        true
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use sill_testkit::TempDir;

    use super::{Handle, Service, ServiceIo, spawn_with};
    use crate::env::Env;
    use crate::error::ServiceError;
    use crate::restart::{RESTARTS_MAX, Tries};

    const WAIT: Duration = Duration::from_secs(20);

    /// Panics on its first run. Every later run publishes its own number, then answers a command
    /// `n` by publishing `100 + n`.
    #[derive(Debug)]
    enum Flaky {}

    impl Service for Flaky {
        const NAME: &'static str = "flaky";
        type State = u32;
        type Command = u32;
        type Deps = Arc<AtomicU32>;

        #[allow(
            clippy::manual_async_fn,
            reason = "an `async fn` would hold the `&Env` it never reads; the trait wants a 'static future"
        )]
        fn run(
            _env: &Env,
            runs: Arc<AtomicU32>,
            mut io: ServiceIo<Self>,
        ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
            async move {
                let run = runs.fetch_add(1, Ordering::SeqCst) + 1;
                assert!(run > 1, "the first run panics");
                io.state.send_replace(run);
                while let Some(command) = io.commands.recv().await {
                    io.state.send_replace(100 + command);
                }
                Ok(())
            }
        }
    }

    /// Never runs: it panics every time.
    #[derive(Debug)]
    enum Broken {}

    impl Service for Broken {
        const NAME: &'static str = "broken";
        type State = u32;
        type Command = u32;
        type Deps = Arc<AtomicU32>;

        #[allow(
            clippy::manual_async_fn,
            reason = "an `async fn` would hold the `&Env` it never reads; the trait wants a 'static future"
        )]
        fn run(
            _env: &Env,
            runs: Arc<AtomicU32>,
            _io: ServiceIo<Self>,
        ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
            async move {
                runs.fetch_add(1, Ordering::SeqCst);
                panic!("every run panics");
            }
        }
    }

    fn quick() -> Tries {
        Tries::starting_at(Duration::from_millis(5))
    }

    async fn state_is<S: Service<State = u32>>(handle: &mut Handle<S>, want: u32) {
        let reached = tokio::time::timeout(WAIT, handle.state.wait_for(|state| *state == want));
        assert!(reached.await.is_ok(), "the state never became {want}");
    }

    #[tokio::test]
    async fn a_service_that_panics_once_is_running_again_and_takes_commands() {
        let scratch = TempDir::new("service-restart");
        let env = Env::isolated(scratch.path());
        let runs = Arc::new(AtomicU32::new(0));
        let mut handle = spawn_with::<Flaky>(
            &tokio::runtime::Handle::current(),
            &env,
            Arc::clone(&runs),
            quick(),
        );
        state_is(&mut handle, 2).await;
        assert_eq!(runs.load(Ordering::SeqCst), 2, "one panic, one restart");
        assert!(handle.control.send(5).is_ok());
        state_is(&mut handle, 105).await;
    }

    #[tokio::test]
    async fn a_service_that_always_panics_is_given_up_on_after_the_limit() {
        let scratch = TempDir::new("service-giveup");
        let env = Env::isolated(scratch.path());
        let runs = Arc::new(AtomicU32::new(0));
        let handle = spawn_with::<Broken>(
            &tokio::runtime::Handle::current(),
            &env,
            Arc::clone(&runs),
            quick(),
        );
        let limit = RESTARTS_MAX + 1;
        let given_up = async {
            while handle.control.send(0).is_ok() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        assert!(tokio::time::timeout(WAIT, given_up).await.is_ok());
        assert_eq!(
            runs.load(Ordering::SeqCst),
            limit,
            "the first run and the restarts"
        );
    }

    #[tokio::test]
    async fn dropping_every_control_ends_the_run_that_was_going() {
        let scratch = TempDir::new("service-close");
        let env = Env::isolated(scratch.path());
        let runs = Arc::new(AtomicU32::new(1));
        let handle = spawn_with::<Flaky>(
            &tokio::runtime::Handle::current(),
            &env,
            Arc::clone(&runs),
            quick(),
        );
        let mut state = handle.state.clone();
        drop(handle);
        // The run publishes 2, sees its commands close and returns; nothing starts it again.
        let _ = tokio::time::timeout(WAIT, state.wait_for(|state| *state == 2)).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }
}
