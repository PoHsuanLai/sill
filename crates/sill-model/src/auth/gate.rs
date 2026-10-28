//! One check at a time for the lock screen's password field: pure, table-tested.
//!
//! Rules (the user chose no growing delay after failed passwords, so there is no lockout and no
//! wait):
//!
//! - `Submit` while `Open` starts a check (`Check`); while `Checking` it is refused (`Busy`) and
//!   nothing is checked, so a held Return key cannot queue attempts.
//! - `Verdict(Accepted)` resets the count to 0 and says `Accepted`.
//! - `Verdict(Refused)` counts one failure and reopens the gate at once (`Wrong`), however many
//!   came before.
//! - `Verdict(Expired)` (the password was right but has run out) reopens the gate at once and
//!   counts nothing (`Expired`): it is not a wrong password, and it does not unlock.
//! - `Verdict(Unavailable)` is not the person's fault: the gate reopens, nothing is counted
//!   (`Unavailable`).
//! - `Reset` (a new lock) reopens the gate and keeps the count.
//! - A verdict that arrives while not `Checking` (a stale child) changes nothing.
//!
//! PAM's own modules may still pause after a wrong password (`pam_unix`'s fixed failure delay,
//! or `pam_faillock` where the system configures it); that is the login stack's policy, not
//! sill's.

use super::{Attempts, Outcome};

/// The machine's whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Checks {
    /// True while a check runs; a secret can be checked only when it is false (the gate is open).
    pub checking: bool,
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
    /// The password has expired: the person is told so.
    Expired,
    Unavailable,
}

/// `checks` after `input`.
pub fn step(checks: Checks, input: GateIn) -> (Checks, Vec<GateOut>) {
    match (checks.checking, input) {
        (false, GateIn::Submit) => (
            Checks {
                checking: true,
                ..checks
            },
            vec![GateOut::Check],
        ),
        (true, GateIn::Submit) => (checks, vec![GateOut::Busy]),
        (true, GateIn::Verdict(outcome)) => verdict(checks, &outcome),
        (_, GateIn::Reset) => (
            Checks {
                checking: false,
                ..checks
            },
            Vec::new(),
        ),
        (false, GateIn::Verdict(_)) => (checks, Vec::new()),
    }
}

/// A check's verdict, while `Checking`.
fn verdict(checks: Checks, outcome: &Outcome) -> (Checks, Vec<GateOut>) {
    match outcome {
        Outcome::Accepted => (Checks::default(), vec![GateOut::Accepted]),
        Outcome::Unavailable(_) => (
            Checks {
                checking: false,
                ..checks
            },
            vec![GateOut::Unavailable],
        ),
        Outcome::Refused => (
            Checks {
                checking: false,
                failures: Attempts(checks.failures.0.saturating_add(1)),
            },
            vec![GateOut::Wrong],
        ),
        Outcome::Expired => (
            Checks {
                checking: false,
                ..checks
            },
            vec![GateOut::Expired],
        ),
    }
}

#[cfg(test)]
mod tests;
