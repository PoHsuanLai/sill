//! The lock screen: sill's own, on shell-host's `ext-session-lock-v1` surfaces (design/20
//! §1.9). sill locks on its own Lock action (the launcher row, the power menu, `sill lock`), on
//! logind's `Lock` for this session, and before sleep on `PrepareForSleep(true)`, holding a
//! logind delay inhibitor until the lock reports locked or [`SLEEP_HOLD_MS`] runs out. Never a
//! second lock while one holds.
//!
//! Which signals are sill's depends on the session ([`Host`]): in sill's own session every
//! trigger is; under cosmic-session or nested in another desktop's session (development) only
//! sill's own Lock action is, so sill never locks on another desktop's behalf.

mod cause;
mod live;
pub(crate) mod logind;
mod machine;
pub mod marker;
mod mode;

pub use cause::{LockCause, cause_after, cause_of};
pub use machine::{
    Cause, Hold, LockPhase, Locker, LockerEffect, LockerEvent, Phase, READY_GIVE_UP_MS, Runner,
    SLEEP_HOLD_MS, Triggers, next_deadline, step,
};
pub use mode::{Host, LockerMode, SessionManager};

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::Bus;

/// What the lock screen is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Lock {
    /// The daemon holds shell-host's session lock whenever this is not `Unlocked`.
    pub phase: LockPhase,
    /// What started the current lock; `None` while `Unlocked`, kept from the request that
    /// started the lock until it is unlocked.
    pub cause: Option<LockCause>,
}

/// What the daemon reports about shell-host's session lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellLockEvent {
    /// The compositor granted the lock (`ext_session_lock_v1.locked`): the machine's `Reported`.
    Locked,
    /// Unlocked after a right password: the machine's `Exited`.
    Unlocked,
    /// The compositor ended the lock itself, or refused it because another client holds one
    /// (`finished`): also `Exited`.
    Finished,
    /// The host cannot lock (no `ext-session-lock-v1`): also `Exited`, so the next Lock can try
    /// again.
    Refused,
}

/// What the locker service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockerCommand {
    /// Resolve the mode against this session (at start).
    Configure,
    /// An already resolved mode (tests).
    Use(LockerMode),
    /// Lock now (sill's own Lock action).
    Lock,
    /// What happened to the shell's own lock.
    Shell(ShellLockEvent),
    /// Lock because sill's idle service turned the displays off (`LockCause::IdleScreenOff`).
    /// Counts in every kind of session the idle
    /// service runs in (it runs only in a sill session, or where a test told it to).
    IdleLock,
}

/// Start the locker service on `rt`. Inert until the first `Configure` or `Use`.
pub fn spawn(rt: &Handle, bus: Bus) -> (watch::Receiver<Lock>, Control<LockerCommand>) {
    let (tx, rx) = watch::channel(Lock::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(bus, tx, commands));
    (rx, control)
}
