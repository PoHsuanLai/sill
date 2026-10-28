//! Idleness: the policy, the grace, inhibitors and the pure step that decides when to dim, lock,
//! turn displays off and suspend.

mod command;
mod grace;
mod inhibit;
mod model;
mod policy;
mod step;

pub use command::IdleCommand;
pub use grace::{GraceCase, GraceTicket, GraceVerdict, WakeDelay, grace_verdict, ticket};
pub use inhibit::{
    BusPeer, InhibitCookie, InhibitMsg, InhibitReply, Inhibition, Inhibitor, Inhibitors,
    LogindIdle, inhibit_step, inhibition, logind_idle,
};
pub use model::{DimShown, Idle, IdlePhase};
pub use policy::{
    DimLevel, DimPlan, IdleConfig, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace,
    LockedScreenOff, OffReason, PowerPair, PowerSource, Threshold, Timeout, Watch, power_source,
    resolve_mode, watches,
};
pub use step::{DisplayPower, IdleEffect, IdleEvent, IdleHint, IdleState, LockSeen, PASSIVE, step};
