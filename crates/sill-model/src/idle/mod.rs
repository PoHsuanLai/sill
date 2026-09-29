//! Idleness: the policy, the grace, inhibitors and the pure step that decides when to dim, lock,
//! turn displays off and suspend.

mod command;
mod cosmic;
mod grace;
mod inhibit;
mod model;
mod policy;
mod step;

pub use command::IdleCommand;
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
pub use model::{DimShown, Idle, IdlePhase};
pub use policy::{
    DimLevel, DimPlan, IdleConfig, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace,
    LockView, LockedScreenOff, OffReason, PowerPair, PowerSource, Threshold, Timeout, TimesChoice,
    Watch, effective_times, power_source, resolve_mode, watches,
};
pub use step::{DisplayPower, IdleEffect, IdleEvent, IdleHint, IdleState, LockSeen, PASSIVE, step};
