//! The lock grace (`session.lock_grace_s`): the one pure rule that may open sill's lock screen
//! without a password, and the ticket that carries
//! its answer to the auth service.
//!
//! The rule: `NoPassword` exactly when **all** hold, else `Ask`:
//! - the lock's cause is [`LockCause::IdleScreenOff`] (sill's own idle path turned the displays
//!   off and locked; never a lock someone asked for, a logind `Lock`, a sleep, a re-lock after a
//!   crash);
//! - the grace is not zero;
//! - no sleep began since the displays went off;
//! - the wake came strictly within the grace of the displays going off (`woke < grace`).
//!
//! The wake is the compositor's `ext_idle_notification_v1.resumed` (real input); a display
//! turning on for any other reason (a hotplug, a resume) is not a wake.

use sill_model::locker::LockCause;

use super::policy::{IdleSecs, LockGrace};

/// Milliseconds from the displays going off (the idle lock's instant) to the wake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WakeDelay(pub u64);

/// Whether logind's `PrepareForSleep(true)` came since the displays went off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SleepSince {
    #[default]
    NoSleep,
    Slept,
}

/// Everything the rule reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraceCase {
    /// `None`: nothing is locked, or the cause is unknown (both ask).
    pub cause: Option<LockCause>,
    pub grace: LockGrace,
    pub woke: WakeDelay,
    pub sleep: SleepSince,
}

/// The rule's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraceVerdict {
    /// Open the lock screen without the password.
    NoPassword,
    /// The lock screen asks as usual.
    Ask,
}

/// The rule (module doc).
pub fn grace_verdict(case: &GraceCase) -> GraceVerdict {
    let LockGrace(IdleSecs(grace_s)) = case.grace;
    let within = case.woke.0 < u64::from(grace_s) * 1_000;
    match (case.cause, case.sleep, within) {
        (Some(LockCause::IdleScreenOff), SleepSince::NoSleep, true) => GraceVerdict::NoPassword,
        (
            Some(
                LockCause::IdleScreenOff
                | LockCause::Action
                | LockCause::Logind
                | LockCause::Sleep
                | LockCause::Restored,
            )
            | None,
            SleepSince::NoSleep | SleepSince::Slept,
            true | false,
        ) => GraceVerdict::Ask,
    }
}

/// Proof that [`grace_verdict`] said `NoPassword`: the only way to build one is [`ticket`], so
/// `AuthCommand::Grace` cannot be sent by anything that did not run the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraceTicket {
    _minted: (),
}

/// A ticket when `case` earns the grace, else `None`.
pub fn ticket(case: &GraceCase) -> Option<GraceTicket> {
    match grace_verdict(case) {
        GraceVerdict::NoPassword => Some(GraceTicket { _minted: () }),
        GraceVerdict::Ask => None,
    }
}

#[cfg(test)]
#[path = "grace_tests.rs"]
mod tests;
