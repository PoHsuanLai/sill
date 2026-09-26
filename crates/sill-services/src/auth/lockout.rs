//! Failures and backoff for the lock screen's password field: pure, table-tested.
//!
//! Rules (FINDINGS F522; macOS's shape, "Try again in 1 minute" after repeated failures):
//!
//! - `Submit` while `Open` starts a check (`Check`); while `Checking` or `Waiting` it is refused
//!   (`Busy`) and nothing is checked, so a held Return key cannot queue attempts.
//! - `Verdict(Accepted)` resets the count to 0 and says `Accepted`.
//! - `Verdict(Refused)` counts one failure. Up to `free_attempts` failures the gate reopens at
//!   once (`Wrong`); past it the gate waits (`Wrong`, then `LockedOut { until }`), the wait
//!   `first_wait_ms` doubled for every failure past the first locked-out one, capped at
//!   `max_wait_ms` ([`wait_after`]).
//! - `Verdict(Unavailable)` is not the person's fault: the gate reopens, nothing is counted
//!   (`Unavailable`).
//! - `Tick(now)` at or past a wait's `until` reopens the gate (`Reopened`); the count stays.
//! - `Reset` (a new lock) reopens an `Open` or `Checking` gate but never shortens a wait, and
//!   keeps the count: locking again must not undo a lockout.
//! - A verdict that arrives while not `Checking` (a stale child) changes nothing.

use crate::units::Stamp;

use super::types::{Attempts, Outcome};

/// The numbers. Not settings (FINDINGS F522 says why): a lockout is a security decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockoutPolicy {
    /// Failures allowed before any wait.
    pub free_attempts: u32,
    /// The first wait.
    pub first_wait_ms: u64,
    /// The longest wait.
    pub max_wait_ms: u64,
}

impl LockoutPolicy {
    /// Five tries, then a minute, doubling to fifteen.
    pub const DEFAULT: LockoutPolicy = LockoutPolicy {
        free_attempts: 5,
        first_wait_ms: 60_000,
        max_wait_ms: 900_000,
    };
}

/// Whether a secret can be checked now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Gate {
    #[default]
    Open,
    Checking,
    Waiting {
        until: Stamp,
    },
}

/// The machine's whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Lockout {
    pub gate: Gate,
    pub failures: Attempts,
}

/// One input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockoutIn {
    Submit(Stamp),
    Verdict(Outcome, Stamp),
    Tick(Stamp),
    Reset,
}

/// One thing the service must do or publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockoutOut {
    /// Run the check.
    Check,
    /// A submit arrived while checking or waiting: ignored.
    Busy,
    Accepted,
    Wrong,
    LockedOut {
        until: Stamp,
    },
    Unavailable,
    /// A wait ran out.
    Reopened,
}

/// `lockout` after `input` under `policy`.
pub fn step(
    lockout: Lockout,
    input: LockoutIn,
    policy: &LockoutPolicy,
) -> (Lockout, Vec<LockoutOut>) {
    todo!("M11 fill: a-lock ({lockout:?} {input:?} {policy:?})")
}

/// The wait after the `failures`-th failure: none up to `free_attempts`, then `first_wait_ms`,
/// doubling per failure, capped at `max_wait_ms`.
pub fn wait_after(failures: Attempts, policy: &LockoutPolicy) -> Option<u64> {
    todo!("M11 fill: a-lock ({failures:?} {policy:?})")
}

/// When `lockout` next wants a `Tick`.
pub fn next_deadline(lockout: &Lockout) -> Option<Stamp> {
    match lockout.gate {
        Gate::Waiting { until } => Some(until),
        Gate::Open | Gate::Checking => None,
    }
}

#[cfg(test)]
#[path = "lockout_tests.rs"]
mod tests;
