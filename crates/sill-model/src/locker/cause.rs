//! Why the session is locked: the one fact the lock grace turns on. Since sill owns idle, every
//! lock has exactly one known cause, set by
//! whichever request moved the lock from `Unlocked` to `Locking` and kept until it is
//! `Unlocked` again: a later request while locked never changes it (so an idle screen-off over
//! a lock someone asked for keeps `Action`, and the grace never applies to it).

use serde::{Deserialize, Serialize};

use super::{Cause, LockPhase, LockerEvent};

/// What started the current lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockCause {
    /// sill's own idle path turned the displays off and locked (`idle.lock = "screen_off"`).
    /// The only cause `session.lock_grace_s` applies to.
    IdleScreenOff,
    /// sill's Lock: ⌃⌘Q or `sill lock`, the launcher row, the power menu.
    Action,
    /// logind's `Lock` for this session: a hand-typed `loginctl lock-session`,
    /// `HandleLidSwitch=lock`, or an idle daemon someone started by hand.
    Logind,
    /// logind's `PrepareForSleep(true)`: a lid close set to suspend, sill's idle suspend,
    /// `systemctl suspend`, the power menu's Sleep.
    Sleep,
    /// A sill that died while its own lock held left the marker, and the new one re-locked.
    Restored,
}

/// The cause to publish after one step of the locker's machine: `None` once unlocked; `why`
/// when the step moved the lock off `Unlocked`; else the cause already kept (a request while
/// locked never changes it).
pub fn cause_after(
    kept: Option<LockCause>,
    was: LockPhase,
    now: LockPhase,
    why: Option<LockCause>,
) -> Option<LockCause> {
    match (was, now) {
        (_, LockPhase::Unlocked) => None,
        (LockPhase::Unlocked, LockPhase::Locking | LockPhase::Locked) => why.or(kept),
        (LockPhase::Locking | LockPhase::Locked, LockPhase::Locking | LockPhase::Locked) => kept,
    }
}

/// The cause a machine event carries if it starts a lock.
pub fn cause_of(event: &LockerEvent) -> Option<LockCause> {
    match event {
        LockerEvent::Requested(Cause::Action, _) => Some(LockCause::Action),
        LockerEvent::Requested(Cause::Logind, _) => Some(LockCause::Logind),
        LockerEvent::Requested(Cause::Idle, _) => Some(LockCause::IdleScreenOff),
        LockerEvent::SleepStarting(_) => Some(LockCause::Sleep),
        LockerEvent::Configured(..)
        | LockerEvent::Woke
        | LockerEvent::Reported
        | LockerEvent::Exited
        | LockerEvent::InhibitorRefused
        | LockerEvent::Tick(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{LockCause, cause_after};
    use crate::locker::LockPhase::{Locked, Locking, Unlocked};

    #[test]
    fn the_first_cause_stands_until_unlocked() {
        use LockCause::{Action, IdleScreenOff, Sleep};
        let cases = [
            (
                None,
                Unlocked,
                Locking,
                Some(IdleScreenOff),
                Some(IdleScreenOff),
            ),
            (None, Unlocked, Locked, Some(Action), Some(Action)),
            (
                Some(Action),
                Locked,
                Locked,
                Some(IdleScreenOff),
                Some(Action),
            ),
            (
                Some(IdleScreenOff),
                Locking,
                Locked,
                None,
                Some(IdleScreenOff),
            ),
            (Some(Action), Locking, Locked, Some(Sleep), Some(Action)),
            (Some(Action), Locked, Unlocked, None, None),
            (None, Unlocked, Unlocked, Some(Action), None),
        ];
        for (kept, was, now, why, want) in cases {
            assert_eq!(
                cause_after(kept, was, now, why),
                want,
                "{kept:?} {was:?} {now:?}"
            );
        }
    }
}
