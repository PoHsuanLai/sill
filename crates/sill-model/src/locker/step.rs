//! The locker state's pure transition.

use super::{
    Hold, LockCause, LockPhase, Locker, LockerEffect, LockerEvent, READY_GIVE_UP_MS, SLEEP_HOLD_MS,
};
use ds_core::time::stamp::Stamp;

/// `locker` after `event`, and the effects to perform.
pub fn step(locker: Locker, event: LockerEvent) -> (Locker, Vec<LockerEffect>) {
    match event {
        LockerEvent::Configured { own, follow_logind } => configured(locker, own, follow_logind),
        LockerEvent::Requested(cause, now) => requested(locker, cause, now),
        LockerEvent::SleepStarting(now) => sleep_starting(locker, now),
        LockerEvent::Woke => arm(locker),
        LockerEvent::Reported => settle(locked_if_locking(locker)),
        LockerEvent::Exited => settle(Locker {
            phase: LockPhase::Unlocked,
            give_up: None,
            ..locker
        }),
        LockerEvent::InhibitorRefused => (
            Locker {
                hold: Hold::Released,
                ..locker
            },
            Vec::new(),
        ),
        LockerEvent::Tick(now) => tick(locker, now),
    }
}

/// The next time [`step`] wants a `Tick`, if any.
pub fn next_deadline(locker: &Locker) -> Option<Stamp> {
    let until = match locker.hold {
        Hold::Delaying { until } => Some(until),
        Hold::Released | Hold::Armed => None,
    };
    locker.give_up.into_iter().chain(until).min()
}

/// Whether logind's sleep is ours to delay: our own lock, in a session whose signals we follow.
fn guards_sleep(locker: &Locker) -> bool {
    locker.own && locker.follow_logind
}

fn configured(locker: Locker, own: bool, follow_logind: bool) -> (Locker, Vec<LockerEffect>) {
    let next = Locker {
        own,
        follow_logind,
        ..locker
    };
    if guards_sleep(&next) {
        return arm(next);
    }
    match next.hold {
        Hold::Released => (next, Vec::new()),
        Hold::Armed | Hold::Delaying { .. } => (
            Locker {
                hold: Hold::Released,
                ..next
            },
            vec![LockerEffect::ReleaseInhibitor],
        ),
    }
}

/// Take the inhibitor if the lock guards sleep and none is held.
fn arm(locker: Locker) -> (Locker, Vec<LockerEffect>) {
    match (guards_sleep(&locker), locker.hold) {
        (true, Hold::Released) => (
            Locker {
                hold: Hold::Armed,
                ..locker
            },
            vec![LockerEffect::TakeInhibitor],
        ),
        _ => (locker, Vec::new()),
    }
}

fn requested(locker: Locker, cause: LockCause, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    let ours = cause != LockCause::Logind || locker.follow_logind;
    if ours && locker.own && locker.phase == LockPhase::Unlocked {
        return start(locker, now);
    }
    (locker, Vec::new())
}

fn start(locker: Locker, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    (
        Locker {
            phase: LockPhase::Locking,
            give_up: Some(now.after(READY_GIVE_UP_MS)),
            ..locker
        },
        Vec::new(),
    )
}

fn sleep_starting(locker: Locker, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    if !guards_sleep(&locker) {
        return (locker, Vec::new());
    }
    let (started, mut effects) = match locker.phase {
        LockPhase::Unlocked => start(locker, now),
        LockPhase::Locking | LockPhase::Locked => (locker, Vec::new()),
    };
    let delaying = match started.hold {
        Hold::Armed => Locker {
            hold: Hold::Delaying {
                until: now.after(SLEEP_HOLD_MS),
            },
            ..started
        },
        Hold::Released | Hold::Delaying { .. } => started,
    };
    let (settled, released) = settle(delaying);
    effects.extend(released);
    (settled, effects)
}

fn locked_if_locking(locker: Locker) -> Locker {
    match locker.phase {
        LockPhase::Locking => Locker {
            phase: LockPhase::Locked,
            give_up: None,
            ..locker
        },
        LockPhase::Unlocked | LockPhase::Locked => locker,
    }
}

/// Let a waiting sleep go once there is nothing left to wait for: the lock is in place, or
/// the lock has ended.
fn settle(locker: Locker) -> (Locker, Vec<LockerEffect>) {
    match (locker.hold, locker.phase) {
        (Hold::Delaying { .. }, LockPhase::Locked | LockPhase::Unlocked) => (
            Locker {
                hold: Hold::Released,
                ..locker
            },
            vec![LockerEffect::ReleaseInhibitor],
        ),
        _ => (locker, Vec::new()),
    }
}

fn tick(locker: Locker, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    let gave_up = matches!(locker.give_up, Some(give_up) if now >= give_up);
    let hold_expired = matches!(locker.hold, Hold::Delaying { until } if now >= until);
    let next = if gave_up {
        locked_if_locking(locker)
    } else {
        locker
    };
    if hold_expired {
        return (
            Locker {
                hold: Hold::Released,
                ..next
            },
            vec![LockerEffect::ReleaseInhibitor],
        );
    }
    settle(next)
}
