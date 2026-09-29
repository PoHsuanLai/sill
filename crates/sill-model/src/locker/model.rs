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
use crate::units::Stamp;

/// How long the lock may take to report locked before it is taken as locked anyway.
pub const READY_GIVE_UP_MS: u64 = 10_000;

/// How long sleep waits for the lock at most: under logind's default `InhibitDelayMaxSec` (5 s),
/// so it is sill, not logind, that lets go.
pub const SLEEP_HOLD_MS: u64 = 4_000;

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

/// Who carries out a lock, once the setting is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Runner {
    /// Nothing locks.
    #[default]
    Off,
    /// sill's own lock screen.
    Own,
}

/// Whether logind's `Lock` and `PrepareForSleep` start a lock, or only sill's own Lock does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Triggers {
    /// A sill session: logind's signals are ours to act on.
    FollowLogind,
    /// Another desktop owns this logind session (sill nested during development, or
    /// cosmic-session): its signals are not ours.
    #[default]
    IgnoreLogind,
}

/// Why a lock was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cause {
    /// sill's Lock: the launcher row, the power menu, `sill lock`.
    Action,
    /// logind's `Lock` signal for this session (`loginctl lock-session`, Super+Escape).
    Logind,
    /// sill's own idle service turned the displays off.
    Idle,
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
    pub runner: Runner,
    pub triggers: Triggers,
    pub phase: Phase,
    pub hold: Hold,
}

/// [`LockPhase`] with the deadline the machine needs while locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    #[default]
    Unlocked,
    Locking {
        give_up: Stamp,
    },
    Locked,
}

impl Phase {
    /// The published form.
    pub fn public(self) -> LockPhase {
        match self {
            Phase::Unlocked => LockPhase::Unlocked,
            Phase::Locking { .. } => LockPhase::Locking,
            Phase::Locked => LockPhase::Locked,
        }
    }
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
