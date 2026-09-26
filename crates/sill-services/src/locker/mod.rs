//! The lock screen, borrowed (design/20 §1.9: an existing `ext-session-lock-v1` locker at M7,
//! sill's own at M11). sill starts the locker `session.locker` names ([`LockerChoice`]) on its
//! own Lock action (the launcher row, the power menu, `sill lock`), on logind's `Lock` for this
//! session, and before sleep on `PrepareForSleep(true)`, holding a logind delay inhibitor until
//! the locker reports locked or [`SLEEP_HOLD_MS`] runs out. Never a second locker while one
//! runs; unlocking is the locker's business, and sill only notices it exit.
//!
//! Which signals are sill's depends on the session ([`Host`]): in sill's own session every
//! trigger is; under cosmic-session a Lock action is handed to logind for COSMIC's resident
//! locker; nested in another desktop's session (development) only sill's own Lock action is,
//! so sill never locks on the host desktop's behalf.

mod child;
mod choice;
mod live;
mod logind;
mod machine;

pub use choice::{
    COSMIC_LOCKED_LINE, GRACE_MS, Host, KnownLocker, LockerChoice, LockerMode, LockerPlan,
    NO_SYSTEM_BUS, Readiness, SessionManager, resolve,
};
pub use machine::{
    Cause, Hold, LockPhase, Locker, LockerEffect, LockerEvent, Phase, READY_GIVE_UP_MS, Runner,
    SLEEP_HOLD_MS, Triggers, next_deadline, step,
};

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::programs::Pid;

/// What the lock screen is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Lock {
    pub phase: LockPhase,
    /// The running locker's process, once there is one.
    pub pid: Option<Pid>,
    /// Who draws the lock screen in the current mode. With `Shell`, the daemon holds
    /// shell-host's session lock whenever `phase` is not `Unlocked` (M11, FINDINGS F524).
    pub drawer: LockDrawer,
}

/// Who draws the lock screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockDrawer {
    /// A locker program (M7's borrowed ones), or nobody when the mode is off.
    #[default]
    Program,
    /// sill's own lock surfaces (`session.locker = "sill"`).
    Shell,
}

/// What the daemon reports about shell-host's session lock (`session.locker = "sill"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellLockEvent {
    /// The compositor granted the lock (`ext_session_lock_v1.locked`): the machine's `Reported`.
    Locked,
    /// The lock ended: unlocked after a right password, or the compositor ended it
    /// (`finished`); the machine's `Exited`.
    Finished,
    /// The host cannot lock (no `ext-session-lock-v1`, or a lock already held by another
    /// client): also `Exited`, so the next Lock can try again.
    Refused,
}

/// What the locker service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockerCommand {
    /// The `session.locker` setting (at start and on every change): resolved against this
    /// session and what is installed.
    Configure(LockerChoice),
    /// An already resolved mode (tests: a stand-in locker on the nested compositor).
    Use(LockerMode),
    /// Lock now (sill's own Lock action).
    Lock,
    /// What happened to the shell's own lock (M11).
    Shell(ShellLockEvent),
}

/// Start the locker service on `rt`. Inert until the first `Configure` or `Use`.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Lock>, Control<LockerCommand>) {
    let (tx, rx) = watch::channel(Lock::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
