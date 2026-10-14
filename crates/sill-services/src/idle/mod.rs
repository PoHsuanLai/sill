//! sill's own idle service: the dim, the displays off, the lock at screen off and the idle
//! suspend (the user's pick). Owning idle is how sill knows why it
//! locked ([`sill_model::locker::LockCause`]), which the lock grace (`session.lock_grace_s`,
//! `grace_verdict`) needs.
//!
//! **Passive until the daemon's first [`IdleCommand`]**, and `Off` (nothing held, nothing
//! touched) anywhere but a sill session (`resolve_mode`), so no
//! test and no nested run ever dims, powers off or suspends the real session's displays. Live
//! tests name a nested compositor and private buses in their [`Env`](crate::env::Env).
//!
//! Pure parts: `resolve_mode`, `power_source`, `watches`,
//! `inhibit_step`, `logind_idle`, `inhibition`,
//! `grace_verdict` and the machine `step`.

mod backend;

use backend::Reach;
use sill_model::idle::{Idle, IdleCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::IdleLinks;

/// sill's own idle as a [`Service`], on `env`'s compositor and buses, reading and driving the
/// services in its [`IdleLinks`]. Passive until the first command.
#[derive(Debug)]
pub enum IdleService {}

impl Service for IdleService {
    const NAME: &'static str = "idle";
    type State = Idle;
    type Command = IdleCommand;
    type Deps = IdleLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = Reach::of(env);
        async move {
            backend::run(reach, deps, io.commands, io.state).await;
            Ok(())
        }
    }
}
