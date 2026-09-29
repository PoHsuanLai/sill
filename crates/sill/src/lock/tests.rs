use sill_ipc::{LockReport, ReportLockPhase};
use sill_services::auth::AuthFault;
use sill_services::{Auth, AuthPhase, Lock, LockPhase};

use super::{LockWish, lock_wish, report_of};

fn lock(phase: LockPhase) -> Lock {
    Lock { phase, cause: None }
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
        (LockPhase::Unlocked, AuthPhase::Idle, LockWish::Released),
        (LockPhase::Locking, AuthPhase::Idle, LockWish::Held),
        (LockPhase::Locked, AuthPhase::Wrong, LockWish::Held),
        (LockPhase::Locked, AuthPhase::Checking, LockWish::Held),
        (
            LockPhase::Locked,
            AuthPhase::Unavailable(AuthFault::TimedOut),
            LockWish::Held,
        ),
        (LockPhase::Locked, AuthPhase::Accepted, LockWish::Released),
    ];
    for (phase, auth_phase, want) in cases {
        assert_eq!(
            lock_wish(&lock(phase), &auth(auth_phase.clone())),
            want,
            "{phase:?} {auth_phase:?}"
        );
    }
}

#[test]
fn the_report_mirrors_the_locker() {
    assert_eq!(
        report_of(&lock(LockPhase::Locking)),
        LockReport {
            phase: ReportLockPhase::Locking,
            cause: None,
        }
    );
    assert_eq!(
        report_of(&Lock::default()),
        LockReport {
            phase: ReportLockPhase::Unlocked,
            cause: None,
        }
    );
    let idle = Lock {
        cause: Some(sill_services::LockCause::IdleScreenOff),
        ..lock(LockPhase::Locked)
    };
    assert_eq!(
        report_of(&idle).cause,
        Some(sill_ipc::ReportLockCause::IdleScreenOff)
    );
}
