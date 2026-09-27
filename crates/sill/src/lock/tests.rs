use sill_ipc::{LockReport, ReportDrawer, ReportLockPhase};
use sill_services::auth::AuthFault;
use sill_services::{Auth, AuthPhase, Lock, LockDrawer, LockPhase};

use super::{LockWish, lock_wish, report_of};

fn lock(phase: LockPhase, drawer: LockDrawer) -> Lock {
    Lock {
        phase,
        pid: None,
        drawer,
        cause: None,
    }
}

fn auth(phase: AuthPhase) -> Auth {
    Auth {
        phase,
        ..Auth::default()
    }
}

#[test]
fn the_wish_follows_the_locker_until_the_password_is_right() {
    let cases = [
        (
            LockPhase::Unlocked,
            LockDrawer::Shell,
            AuthPhase::Idle,
            LockWish::Released,
        ),
        (
            LockPhase::Locking,
            LockDrawer::Shell,
            AuthPhase::Idle,
            LockWish::Held,
        ),
        (
            LockPhase::Locked,
            LockDrawer::Shell,
            AuthPhase::Wrong,
            LockWish::Held,
        ),
        (
            LockPhase::Locked,
            LockDrawer::Shell,
            AuthPhase::Checking,
            LockWish::Held,
        ),
        (
            LockPhase::Locked,
            LockDrawer::Shell,
            AuthPhase::Unavailable(AuthFault::TimedOut),
            LockWish::Held,
        ),
        (
            LockPhase::Locked,
            LockDrawer::Shell,
            AuthPhase::Accepted,
            LockWish::Released,
        ),
        (
            LockPhase::Locked,
            LockDrawer::Program,
            AuthPhase::Idle,
            LockWish::Released,
        ),
    ];
    for (phase, drawer, auth_phase, want) in cases {
        assert_eq!(
            lock_wish(&lock(phase, drawer), &auth(auth_phase.clone())),
            want,
            "{phase:?} {drawer:?} {auth_phase:?}"
        );
    }
}

#[test]
fn the_report_mirrors_the_locker() {
    assert_eq!(
        report_of(&lock(LockPhase::Locking, LockDrawer::Shell)),
        LockReport {
            phase: ReportLockPhase::Locking,
            drawer: ReportDrawer::Shell,
        }
    );
    assert_eq!(
        report_of(&Lock::default()),
        LockReport {
            phase: ReportLockPhase::Unlocked,
            drawer: ReportDrawer::Program,
        }
    );
}
