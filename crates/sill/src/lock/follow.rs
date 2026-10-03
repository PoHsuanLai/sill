//! What the follower sends, pure: [`super::lock_wish`] plus one guard. After a right password
//! the wish is `Released`, and it must stay so until the locker has seen the lock end: the auth
//! service is reset on `Unlocked` at about the moment the locker hears it, and a follower that
//! saw the reset first (`Idle` beside a still `Locked` locker) would otherwise ask for the lock
//! again. So an `Accepted` while the locker holds latches [`Latch::Unlocking`], which only the
//! locker's `Unlocked` (the lock ended) or `Locking` (a new lock asked for) clears.

use sill_model::auth::{Auth, AuthPhase};
use sill_model::locker::{Lock, LockPhase};

use super::{LockWish, lock_wish};

/// Whether an unlock is on its way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Latch {
    #[default]
    Free,
    Unlocking,
}

/// How a `Released` is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Send {
    /// At once.
    Now,
    /// After `UNLOCK_BACKSTOP_MS`, if nothing changes meanwhile: the lock surfaces unlock first,
    /// and this is only the backstop.
    AfterBackstop,
}

/// The latch and the wish after `lock` and `auth`, and how to send the wish.
pub fn follow(latch: Latch, lock: &Lock, auth: &Auth) -> (Latch, LockWish, Send) {
    let latch = match (latch, lock.phase, &auth.phase) {
        (_, LockPhase::Locked, AuthPhase::Accepted) => Latch::Unlocking,
        (Latch::Unlocking, LockPhase::Unlocked | LockPhase::Locking, _) => Latch::Free,
        (latch, _, _) => latch,
    };
    let wish = match latch {
        Latch::Unlocking => LockWish::Released,
        Latch::Free => lock_wish(lock, auth),
    };
    let send = match (latch, wish) {
        (Latch::Unlocking, LockWish::Released) => Send::AfterBackstop,
        _ => Send::Now,
    };
    (latch, wish, send)
}

#[cfg(test)]
mod tests {
    use sill_model::auth::{Auth, AuthPhase};
    use sill_model::locker::{Lock, LockPhase};

    use super::{Latch, Send, follow};
    use crate::lock::LockWish;

    fn step(latch: Latch, phase: LockPhase, auth: AuthPhase) -> (Latch, LockWish, Send) {
        let lock = Lock { phase, cause: None };
        let auth = Auth {
            phase: auth,
            ..Auth::default()
        };
        follow(latch, &lock, &auth)
    }

    #[test]
    fn an_unlock_never_turns_back_into_a_lock() {
        use AuthPhase::{Accepted, Idle, Wrong};
        use LockPhase::{Locked, Locking, Unlocked};
        let cases = [
            (
                Latch::Free,
                Locking,
                Idle,
                (Latch::Free, LockWish::Held, Send::Now),
            ),
            (
                Latch::Free,
                Locked,
                Wrong,
                (Latch::Free, LockWish::Held, Send::Now),
            ),
            (
                Latch::Free,
                Locked,
                Accepted,
                (Latch::Unlocking, LockWish::Released, Send::AfterBackstop),
            ),
            // The auth service's reset seen before the locker's unlock: still released.
            (
                Latch::Unlocking,
                Locked,
                Idle,
                (Latch::Unlocking, LockWish::Released, Send::AfterBackstop),
            ),
            (
                Latch::Unlocking,
                Unlocked,
                Idle,
                (Latch::Free, LockWish::Released, Send::Now),
            ),
            // A stale `Accepted` beside a new lock is not an unlock.
            (
                Latch::Free,
                Locking,
                Accepted,
                (Latch::Free, LockWish::Held, Send::Now),
            ),
            // A new lock asked for right away clears the latch.
            (
                Latch::Unlocking,
                Locking,
                Idle,
                (Latch::Free, LockWish::Held, Send::Now),
            ),
        ];
        for (latch, phase, auth, want) in cases {
            assert_eq!(
                step(latch, phase, auth.clone()),
                want,
                "{latch:?} {phase:?} {auth:?}"
            );
        }
    }
}
