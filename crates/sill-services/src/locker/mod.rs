//! The lock screen: sill's own, on shell-host's `ext-session-lock-v1` surfaces (design/20
//! §1.9). sill locks on its own Lock action (the launcher row, the power menu, `sill lock`), on
//! logind's `Lock` for this session, and before sleep on `PrepareForSleep(true)`, holding a
//! logind delay inhibitor until the lock reports locked or `SLEEP_HOLD_MS` runs out. Never a
//! second lock while one holds.
//!
//! Which signals are sill's depends on the session (`sill_model::locker::Host`): in sill's own session every
//! trigger is; under cosmic-session or nested in another desktop's session (development) only
//! sill's own Lock action is, so sill never locks on another desktop's behalf.

mod backend;

use sill_model::locker::{Lock, LockerCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub(crate) use backend::session_path;

/// sill's own lock screen as a [`Service`], logind on `env`'s system bus, the crash marker in
/// its runtime directory. Inert until the first `Configure` or `Use`.
#[derive(Debug)]
pub enum LockerService {}

impl Service for LockerService {
    const NAME: &'static str = "locker";
    type State = Lock;
    type Command = LockerCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.system.clone();
        let runtime = env.dirs.runtime.clone();
        let processes = env.processes.clone();
        async move {
            backend::run(bus, runtime, processes, io.state, io.commands).await;
            Ok(())
        }
    }
}
