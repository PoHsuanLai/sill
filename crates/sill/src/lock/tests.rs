use sill_model::auth::{Auth, AuthFault, AuthPhase};
use sill_model::locker::{Lock, LockPhase};

use super::{LockWish, lock_wish};

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
        // A verdict left over from the lock before is no unlock of a lock not yet taken: the
        // stall where a sleep's lock stayed `locking` until the locker's 10 s give-up.
        (LockPhase::Locking, AuthPhase::Accepted, LockWish::Held),
        (LockPhase::Unlocked, AuthPhase::Accepted, LockWish::Released),
    ];
    for (phase, auth_phase, want) in cases {
        assert_eq!(
            lock_wish(&lock(phase), &auth(auth_phase.clone())),
            want,
            "{phase:?} {auth_phase:?}"
        );
    }
}
