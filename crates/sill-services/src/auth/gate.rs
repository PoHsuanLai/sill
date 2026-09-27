//! One check at a time for the lock screen's password field: pure, table-tested.
//!
//! Rules (FINDINGS F542, amended by F862: the user chose no growing delay after failed
//! passwords, so there is no lockout and no wait):
//!
//! - `Submit` while `Open` starts a check (`Check`); while `Checking` it is refused (`Busy`) and
//!   nothing is checked, so a held Return key cannot queue attempts.
//! - `Verdict(Accepted)` resets the count to 0 and says `Accepted`.
//! - `Verdict(Refused)` counts one failure and reopens the gate at once (`Wrong`), however many
//!   came before.
//! - `Verdict(Unavailable)` is not the person's fault: the gate reopens, nothing is counted
//!   (`Unavailable`).
//! - `Reset` (a new lock) reopens the gate and keeps the count.
//! - A verdict that arrives while not `Checking` (a stale child) changes nothing.
//!
//! PAM's own modules may still pause after a wrong password (`pam_unix`'s fixed failure delay,
//! or `pam_faillock` where the system configures it); that is the login stack's policy, not
//! sill's.

use super::types::{Attempts, Outcome};

/// Whether a secret can be checked now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Gate {
    #[default]
    Open,
    Checking,
}

/// The machine's whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Checks {
    pub gate: Gate,
    pub failures: Attempts,
}

/// One input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GateIn {
    Submit,
    Verdict(Outcome),
    Reset,
}

/// One thing the service must do or publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateOut {
    /// Run the check.
    Check,
    /// A submit arrived while checking: ignored.
    Busy,
    Accepted,
    Wrong,
    Unavailable,
}

/// `checks` after `input`.
pub fn step(checks: Checks, input: GateIn) -> (Checks, Vec<GateOut>) {
    match (checks.gate, input) {
        (Gate::Open, GateIn::Submit) => (
            Checks {
                gate: Gate::Checking,
                ..checks
            },
            vec![GateOut::Check],
        ),
        (Gate::Checking, GateIn::Submit) => (checks, vec![GateOut::Busy]),
        (Gate::Checking, GateIn::Verdict(outcome)) => verdict(checks, &outcome),
        (Gate::Open | Gate::Checking, GateIn::Reset) => (
            Checks {
                gate: Gate::Open,
                ..checks
            },
            Vec::new(),
        ),
        (Gate::Open, GateIn::Verdict(_)) => (checks, Vec::new()),
    }
}

/// A check's verdict, while `Checking`.
fn verdict(checks: Checks, outcome: &Outcome) -> (Checks, Vec<GateOut>) {
    match outcome {
        Outcome::Accepted => (Checks::default(), vec![GateOut::Accepted]),
        Outcome::Unavailable(_) => (
            Checks {
                gate: Gate::Open,
                ..checks
            },
            vec![GateOut::Unavailable],
        ),
        Outcome::Refused => (
            Checks {
                gate: Gate::Open,
                failures: Attempts(checks.failures.0.saturating_add(1)),
            },
            vec![GateOut::Wrong],
        ),
    }
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
