//! Re-locking after a crash (M11, FINDINGS F544): with `session.locker = "sill"` the lock
//! surfaces are the shell's own, and a compositor keeps a session locked when its locker dies
//! (cosmic-comp: F381), so a sill that dies while locked leaves a locked, blank session.
//! `dist/sill-session` restarts `sill run`; the new daemon must then lock again at once so the
//! person gets a prompt. No protocol lets a client ask whether the session is locked, so the
//! service keeps a marker file while its own lock is held: written when the lock is `Locked`,
//! removed when it ends. A starting service in `Own` mode that finds the marker locks at once. A
//! stale marker (the compositor restarted too) costs one needless password prompt, never an
//! unlocked session.

use std::path::{Path, PathBuf};

use super::choice::LockerMode;

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

/// `$XDG_RUNTIME_DIR/sill/locked` for `runtime_dir`.
pub fn marker_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("sill").join("locked")
}

/// `LockNow` exactly when `mode` is `Own` and the marker is `Present`; a program locker has its
/// own process to find (`programs::running`), and `Off`/`Delegate` never lock.
pub fn restore_on_start(mode: &LockerMode, marker: LockMarker) -> Restore {
    match (mode, marker) {
        (LockerMode::Own { .. }, LockMarker::Present) => Restore::LockNow,
        (LockerMode::Own { .. }, LockMarker::Absent)
        | (LockerMode::Off | LockerMode::Delegate | LockerMode::Run { .. }, _) => Restore::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{LockMarker, Restore, marker_path, restore_on_start};
    use crate::locker::choice::LockerMode;
    use crate::locker::machine::Triggers;

    #[test]
    fn a_marker_relocks_only_the_own_lock() {
        let own = LockerMode::Own {
            triggers: Triggers::FollowLogind,
        };
        let cases = [
            (own.clone(), LockMarker::Present, Restore::LockNow),
            (own, LockMarker::Absent, Restore::Nothing),
            (LockerMode::Off, LockMarker::Present, Restore::Nothing),
            (LockerMode::Delegate, LockMarker::Present, Restore::Nothing),
        ];
        for (mode, marker, want) in cases {
            assert_eq!(restore_on_start(&mode, marker), want, "{mode:?} {marker:?}");
        }
        assert_eq!(
            marker_path(Path::new("/run/user/1000")),
            PathBuf::from("/run/user/1000/sill/locked")
        );
    }
}
