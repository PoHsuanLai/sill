//! Re-locking after a crash: the lock surfaces are the shell's own, and a compositor keeps a
//! session locked when its locker dies, so a sill that dies while locked leaves
//! a locked, blank session.
//! `dist/sill-session` restarts `sill run`; the new daemon must then lock again at once so the
//! person gets a prompt. No protocol lets a client ask whether the session is locked, so the
//! service keeps a marker file while its own lock is held: written as soon as a lock is asked
//! for (before the compositor is asked to cover the session, so a crash in the gap still
//! relocks), removed when the lock ends. A starting service in `Own` mode that finds the marker locks at once. A
//! stale marker (the compositor restarted too) costs one needless password prompt, never an
//! unlocked session.

use std::path::{Path, PathBuf};

use sill_model::locker::{Host, LockPhase};

/// Whether the marker file exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockMarker {
    Present,
    Absent,
}

/// What a service that has just resolved its mode does about the marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Restore {
    /// Lock now (a `Requested(Action)`).
    LockNow,
    Nothing,
}

/// What the marker file must do when the lock goes from one phase to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerChange {
    /// A lock was asked for: record it before anything covers the session.
    Write,
    /// The lock ended.
    Remove,
    /// The marker is already right.
    Keep,
}

/// The marker's change for the lock going from `was` to `now`.
pub fn marker_change(was: LockPhase, now: LockPhase) -> MarkerChange {
    match (was, now) {
        (LockPhase::Unlocked, LockPhase::Locking | LockPhase::Locked) => MarkerChange::Write,
        (LockPhase::Locking | LockPhase::Locked, LockPhase::Unlocked) => MarkerChange::Remove,
        (LockPhase::Unlocked, LockPhase::Unlocked)
        | (LockPhase::Locking | LockPhase::Locked, LockPhase::Locking | LockPhase::Locked) => {
            MarkerChange::Keep
        }
    }
}

/// `$XDG_RUNTIME_DIR/sill/locked` for `runtime_dir`.
pub fn marker_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("sill").join("locked")
}

/// `LockNow` exactly when sill's own lock runs (`mode` is `Some`) and the marker is `Present`;
/// no mode never locks.
pub fn restore_on_start(mode: Option<Host>, marker: LockMarker) -> Restore {
    match (mode, marker) {
        (Some(_), LockMarker::Present) => Restore::LockNow,
        (Some(_), LockMarker::Absent) | (None, _) => Restore::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{LockMarker, MarkerChange, Restore, marker_change, marker_path, restore_on_start};
    use sill_model::locker::Host;
    use sill_model::locker::LockPhase::{Locked, Locking, Unlocked};

    #[test]
    fn the_marker_is_written_when_a_lock_is_asked_for_and_removed_when_it_ends() {
        use MarkerChange::{Keep, Remove, Write};
        let cases = [
            ("asked for", Unlocked, Locking, Write),
            ("locked without a wait", Unlocked, Locked, Write),
            ("reported locked", Locking, Locked, Keep),
            ("refused while locking", Locking, Unlocked, Remove),
            ("unlocked", Locked, Unlocked, Remove),
            ("idle", Unlocked, Unlocked, Keep),
            ("still locking", Locking, Locking, Keep),
        ];
        for (name, was, now, want) in cases {
            assert_eq!(marker_change(was, now), want, "{name}");
        }
    }

    #[test]
    fn a_marker_relocks_only_the_own_lock() {
        let own = Some(Host::SillSession);
        let cases = [
            (own, LockMarker::Present, Restore::LockNow),
            (own, LockMarker::Absent, Restore::Nothing),
            (None, LockMarker::Present, Restore::Nothing),
        ];
        for (mode, marker, want) in cases {
            assert_eq!(restore_on_start(mode, marker), want, "{mode:?} {marker:?}");
        }
        assert_eq!(
            marker_path(Path::new("/run/user/1000")),
            PathBuf::from("/run/user/1000/sill/locked")
        );
    }
}
