//! `Quit`: ends the shell, except while the session is locked. The lock is the shell's own, so a
//! shell that left would leave the session covered with nobody to take a password.

use sill_model::locker::LockPhase;

/// Whether the shell may end now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaving {
    /// Nothing is covered: the shell may go.
    Allowed,
    /// The session is locked or being locked: it must stay.
    Refused,
}

/// What `Quit` may do at `phase`.
pub fn leaving_at(phase: LockPhase) -> Leaving {
    match phase {
        LockPhase::Unlocked => Leaving::Allowed,
        LockPhase::Locking | LockPhase::Locked => Leaving::Refused,
    }
}

/// What the person who asked is told when `Quit` is refused.
pub const REFUSED: &str = "The session is locked. Unlock it before quitting the shell.";

#[cfg(test)]
mod tests {
    use super::{Leaving, leaving_at};
    use sill_model::locker::LockPhase;

    #[test]
    fn the_shell_may_quit_only_while_nothing_is_covered() {
        const CASES: &[(&str, LockPhase, Leaving)] = &[
            ("unlocked", LockPhase::Unlocked, Leaving::Allowed),
            ("locking", LockPhase::Locking, Leaving::Refused),
            ("locked", LockPhase::Locked, Leaving::Refused),
        ];
        for (name, phase, want) in CASES {
            assert_eq!(leaving_at(*phase), *want, "{name}");
        }
    }
}
