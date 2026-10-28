//! The locker's pure state machine: when to lock, when to hold and release logind's sleep
//! delay, and what the lock screen is doing. `step` is the only place it changes; the
//! live side performs the [`LockerEffect`]s it returns and feeds back what happened.
//!
//! The sleep delay follows logind's rule that a delay inhibitor only delays a sleep it was
//! already held for: it is taken as soon as the lock is configured (and again after every
//! resume), and released once the lock reports locked, the lock ends, or
//! [`SLEEP_HOLD_MS`] runs out, whichever is first.

use serde::{Deserialize, Serialize};

use super::LockCause;
use ds_core::time::stamp::{Ms, Stamp};

/// How long the lock may take to report locked before it is taken as locked anyway.
pub const READY_GIVE_UP_MS: Ms = Ms(10_000);

/// How long sleep waits for the lock at most: under logind's default `InhibitDelayMaxSec` (5 s),
/// so it is sill, not logind, that lets go.
pub const SLEEP_HOLD_MS: Ms = Ms(4_000);

/// What the lock screen is doing, as surfaces and tests see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockPhase {
    #[default]
    Unlocked,
    /// A lock was asked for and has not said locked.
    Locking,
    Locked,
}

/// The sleep delay inhibitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Hold {
    /// None held.
    #[default]
    Released,
    /// Held, waiting for a sleep.
    Armed,
    /// A sleep is waiting on it until the lock reports locked or `until`.
    Delaying { until: Stamp },
}

/// The locker service's whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Locker {
    /// True when sill's own lock screen does the locking; false and nothing locks.
    pub own: bool,
    /// True when logind's `Lock` and `PrepareForSleep` start a lock (a sill session); false when
    /// another desktop owns this logind session and only sill's own Lock does.
    pub follow_logind: bool,
    pub phase: LockPhase,
    /// When a lock that has not said locked is taken as locked anyway; set exactly while
    /// `phase` is `Locking`.
    pub give_up: Option<Stamp>,
    pub hold: Hold,
}

/// What the lock screen is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Lock {
    /// The daemon holds shell-host's session lock whenever this is not `Unlocked`.
    pub phase: LockPhase,
    /// What started the current lock; `None` while `Unlocked`, kept from the request that
    /// started the lock until it is unlocked.
    pub cause: Option<LockCause>,
}
