//! The session lock: when the shell locks, how, and why, as the lock surface and the idle service
//! see it.

mod cause;
mod command;
mod mode;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use cause::{LockCause, cause_after, cause_of};
pub use command::{LockerCommand, LockerEffect, LockerEvent, ShellLockEvent};
pub use mode::{Host, LockerMode};
pub use model::{
    Cause, Hold, Lock, LockPhase, Locker, Phase, READY_GIVE_UP_MS, Runner, SLEEP_HOLD_MS, Triggers,
};
pub use step::{next_deadline, step};
