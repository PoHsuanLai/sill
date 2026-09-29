//! sill's own idle service: the dim, the displays off, the lock at screen off and the idle
//! suspend that cosmic-idle did before (the user's pick). Owning idle is how sill knows why it
//! locked ([`sill_model::locker::LockCause`]), which the lock grace (`session.lock_grace_s`,
//! [`grace_verdict`]) needs.
//!
//! **Passive until the daemon's first [`IdleCommand`]**, and `Off` (nothing held, nothing
//! touched) anywhere but a sill session with no cosmic-idle running ([`resolve_mode`]), so no
//! test and no nested run ever dims, powers off or suspends the real session's displays. Live
//! tests name a nested compositor and private buses in their [`Env`](crate::env::Env).
//!
//! Pure parts: [`resolve_mode`], [`effective_times`], [`power_source`], [`watches`],
//! [`parse_cosmic_value`], [`cosmic_times`], [`inhibit_step`], [`logind_idle`], [`inhibition`],
//! [`grace_verdict`] and the machine [`step`].

mod cosmic;
mod grace;
mod inhibit;
mod live;
mod logind;
mod model;
mod policy;
mod reduce;
mod resolve;
mod screensaver;
mod wl;
mod wl_dispatch;
mod wl_state;

pub use cosmic::{
    COSMIC_IDLE_ID, COSMIC_IDLE_VERSION, CosmicIdle, CosmicIdleTimes, CosmicKey, CosmicValue,
    cosmic_times, parse_cosmic_value,
};
pub use grace::{
    GraceCase, GraceTicket, GraceVerdict, SleepSince, WakeDelay, grace_verdict, ticket,
};
pub use inhibit::{
    BusPeer, InhibitCookie, InhibitMsg, InhibitReply, Inhibition, Inhibitor, Inhibitors,
    LogindIdle, inhibit_step, inhibition, logind_idle,
};
pub use model::{DimShown, Idle, IdleCommand, IdleLinks, IdlePhase};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};
use model::Reach;
pub use policy::{
    DimLevel, DimPlan, IdleConfig, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace,
    LockView, LockedScreenOff, OffReason, PowerPair, PowerSource, Threshold, Timeout, TimesChoice,
    Watch, effective_times, power_source, resolve_mode, watches,
};
pub use reduce::{
    DisplayPower, IdleEffect, IdleEvent, IdleHint, IdleState, LockSeen, PASSIVE, step,
};

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
            live::run(reach, deps, io.commands, io.state).await;
            Ok(())
        }
    }
}
