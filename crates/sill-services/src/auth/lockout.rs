//! Failures and backoff for the lock screen's password field: pure, table-tested.
//!
//! Rules (FINDINGS F542; macOS's shape, "Try again in 1 minute" after repeated failures):
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

/// The numbers. Not settings (FINDINGS F542 says why): a lockout is a security decision.
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
    match (lockout.gate, input) {
        (Gate::Open, LockoutIn::Submit(_)) => (
            Lockout {
                gate: Gate::Checking,
                ..lockout
            },
            vec![LockoutOut::Check],
        ),
        (Gate::Checking | Gate::Waiting { .. }, LockoutIn::Submit(_)) => {
            (lockout, vec![LockoutOut::Busy])
        }
        (Gate::Checking, LockoutIn::Verdict(outcome, at)) => verdict(lockout, &outcome, at, policy),
        (Gate::Waiting { until }, LockoutIn::Tick(now)) if now >= until => (
            Lockout {
                gate: Gate::Open,
                ..lockout
            },
            vec![LockoutOut::Reopened],
        ),
        (Gate::Open | Gate::Checking, LockoutIn::Reset) => (
            Lockout {
                gate: Gate::Open,
                ..lockout
            },
            Vec::new(),
        ),
        (
            Gate::Open | Gate::Waiting { .. },
            LockoutIn::Verdict(..) | LockoutIn::Reset | LockoutIn::Tick(_),
        )
        | (Gate::Checking, LockoutIn::Tick(_)) => (lockout, Vec::new()),
    }
}

/// A check's verdict, while `Checking`.
fn verdict(
    lockout: Lockout,
    outcome: &Outcome,
    at: Stamp,
    policy: &LockoutPolicy,
) -> (Lockout, Vec<LockoutOut>) {
    match outcome {
        Outcome::Accepted => (Lockout::default(), vec![LockoutOut::Accepted]),
        Outcome::Unavailable(_) => (
            Lockout {
                gate: Gate::Open,
                ..lockout
            },
            vec![LockoutOut::Unavailable],
        ),
        Outcome::Refused => {
            let failures = Attempts(lockout.failures.0.saturating_add(1));
            match wait_after(failures, policy) {
                None => (
                    Lockout {
                        gate: Gate::Open,
                        failures,
                    },
                    vec![LockoutOut::Wrong],
                ),
                Some(wait) => {
                    let until = Stamp(at.0.saturating_add(wait));
                    (
                        Lockout {
                            gate: Gate::Waiting { until },
                            failures,
                        },
                        vec![LockoutOut::Wrong, LockoutOut::LockedOut { until }],
                    )
                }
            }
        }
    }
}

/// The wait after the `failures`-th failure: none up to `free_attempts`, then `first_wait_ms`,
/// doubling per failure, capped at `max_wait_ms`.
pub fn wait_after(failures: Attempts, policy: &LockoutPolicy) -> Option<u64> {
    let past = failures
        .0
        .checked_sub(policy.free_attempts)?
        .checked_sub(1)?;
    let doubled = 1_u64
        .checked_shl(past)
        .and_then(|factor| policy.first_wait_ms.checked_mul(factor))
        .unwrap_or(u64::MAX);
    Some(doubled.min(policy.max_wait_ms))
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
