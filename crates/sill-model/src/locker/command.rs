//! What the locker service hears and what it can be told.

use super::{Host, LockCause};
use ds_core::time::stamp::Stamp;

/// One thing the live side learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockerEvent {
    /// The mode resolved.
    Configured {
        /// Sill's own lock screen does the locking.
        own: bool,
        /// logind's `Lock` and `PrepareForSleep` start a lock.
        follow_logind: bool,
    },
    /// A lock was asked for.
    Requested(LockCause, Stamp),
    /// logind's `PrepareForSleep(true)`.
    SleepStarting(Stamp),
    /// logind's `PrepareForSleep(false)`.
    Woke,
    /// The compositor granted the lock.
    Reported,
    /// The lock ended, or could not be taken.
    Exited,
    /// Taking the inhibitor failed.
    InhibitorRefused,
    /// Time passed.
    Tick(Stamp),
}

/// What the live side must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockerEffect {
    /// Take `Inhibit("sleep", "sill", "lock before sleep", "delay")`.
    TakeInhibitor,
    /// Close the inhibitor's descriptor.
    ReleaseInhibitor,
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
    /// An already resolved mode: sill's own lock screen for this kind of session, or `None` to
    /// never lock (tests).
    Use(Option<Host>),
    /// Lock now (sill's own Lock action).
    Lock,
    /// What happened to the shell's own lock.
    Shell(ShellLockEvent),
    /// Lock because sill's idle service turned the displays off (`LockCause::IdleScreenOff`).
    /// Counts in every kind of session the idle
    /// service runs in (it runs only in a sill session, or where a test told it to).
    IdleLock,
}
