//! The locker state's pure transition.

use super::{
    Cause, Hold, Locker, LockerEffect, LockerEvent, Phase, READY_GIVE_UP_MS, Runner, SLEEP_HOLD_MS,
    Triggers,
};
use ds_core::time::stamp::Stamp;

/// `locker` after `event`, and the effects to perform.
pub fn step(locker: Locker, event: LockerEvent) -> (Locker, Vec<LockerEffect>) {
    match event {
        LockerEvent::Configured(runner, triggers) => configured(locker, runner, triggers),
        LockerEvent::Requested(cause, now) => requested(locker, cause, now),
        LockerEvent::SleepStarting(now) => sleep_starting(locker, now),
        LockerEvent::Woke => arm(locker),
        LockerEvent::Reported => settle(Locker {
            phase: locked_if_locking(locker.phase),
            ..locker
        }),
        LockerEvent::Exited => settle(Locker {
            phase: Phase::Unlocked,
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
    let give_up = match locker.phase {
        Phase::Locking { give_up } => Some(give_up),
        Phase::Unlocked | Phase::Locked => None,
    };
    let until = match locker.hold {
        Hold::Delaying { until } => Some(until),
        Hold::Released | Hold::Armed => None,
    };
    give_up.into_iter().chain(until).min()
}

/// Whether logind's sleep is ours to delay: our own lock, in a session whose signals we follow.
fn guards_sleep(locker: &Locker) -> bool {
    locker.runner == Runner::Own && locker.triggers == Triggers::FollowLogind
}

fn configured(locker: Locker, runner: Runner, triggers: Triggers) -> (Locker, Vec<LockerEffect>) {
    let next = Locker {
        runner,
        triggers,
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

fn requested(locker: Locker, cause: Cause, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    let ours = match cause {
        Cause::Action | Cause::Idle => true,
        Cause::Logind => locker.triggers == Triggers::FollowLogind,
    };
    if !ours {
        return (locker, Vec::new());
    }
    match (locker.runner, locker.phase) {
        (Runner::Own, Phase::Unlocked) => start(locker, now),
        _ => (locker, Vec::new()),
    }
}

fn start(locker: Locker, now: Stamp) -> (Locker, Vec<LockerEffect>) {
    (
        Locker {
            phase: Phase::Locking {
                give_up: now.after(READY_GIVE_UP_MS),
            },
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
        Phase::Unlocked => start(locker, now),
        Phase::Locking { .. } | Phase::Locked => (locker, Vec::new()),
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

fn locked_if_locking(phase: Phase) -> Phase {
    match phase {
        Phase::Locking { .. } => Phase::Locked,
        other => other,
    }
}

/// Let a waiting sleep go once there is nothing left to wait for: the lock is in place, or
/// the lock has ended.
fn settle(locker: Locker) -> (Locker, Vec<LockerEffect>) {
    match (locker.hold, locker.phase) {
        (Hold::Delaying { .. }, Phase::Locked | Phase::Unlocked) => (
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
    let phase = match locker.phase {
        Phase::Locking { give_up } if now >= give_up => Phase::Locked,
        other => other,
    };
    let hold_expired = matches!(locker.hold, Hold::Delaying { until } if now >= until);
    let next = Locker { phase, ..locker };
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
