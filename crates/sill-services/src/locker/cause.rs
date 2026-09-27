//! Why the session is locked (sill FINDINGS "sill idle (Q420 B)", F907): the one fact the lock
//! grace turns on. Since sill owns idle (Q420 B), every lock has exactly one known cause, set by
//! whichever request moved the lock from `Unlocked` to `Locking` and kept until it is
//! `Unlocked` again: a later request while locked never changes it (so an idle screen-off over
//! a lock someone asked for keeps `Action`, and the grace never applies to it).

use serde::{Deserialize, Serialize};

/// What started the current lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockCause {
    /// sill's own idle path turned the displays off and locked (`idle.lock = "screen_off"`).
    /// The only cause `session.lock_grace_s` applies to.
    IdleScreenOff,
    /// sill's Lock: ⌃⌘Q or `sill lock`, the launcher row, the power menu.
    Action,
    /// logind's `Lock` for this session: Super+Escape (COSMIC's `LockScreen` system action,
    /// `loginctl lock-session`), a hand-typed `loginctl lock-session`, `HandleLidSwitch=lock`,
    /// or a cosmic-idle someone started by hand.
    Logind,
    /// logind's `PrepareForSleep(true)`: a lid close set to suspend, sill's idle suspend,
    /// `systemctl suspend`, the power menu's Sleep.
    Sleep,
    /// A sill that died while its own lock held left the marker, and the new one re-locked
    /// (F544).
    Restored,
    /// A locker program was already running when sill looked, and was adopted (F386).
    Adopted,
}
