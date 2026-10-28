use LockerEffect::{ReleaseInhibitor, TakeInhibitor};

use super::{
    Hold, LockCause, LockPhase, Locker, LockerEffect, LockerEvent, READY_GIVE_UP_MS, SLEEP_HOLD_MS,
    next_deadline, step,
};
use ds_core::time::stamp::{Ms, Stamp};

const T0: Stamp = Stamp(1_000);

const FOLLOW_LOGIND: bool = true;
const IGNORE_LOGIND: bool = false;

/// An own lock; the give-up deadline is kept in step with a `Locking` phase.
fn runs(follow_logind: bool, phase: LockPhase, hold: Hold) -> Locker {
    Locker {
        own: true,
        follow_logind,
        phase,
        give_up: (phase == LockPhase::Locking).then(|| T0.after(READY_GIVE_UP_MS)),
        hold,
    }
}

fn locking() -> LockPhase {
    LockPhase::Locking
}

fn delaying() -> Hold {
    Hold::Delaying {
        until: T0.after(SLEEP_HOLD_MS),
    }
}

#[test]
fn every_transition() {
    let own = |phase, hold| runs(FOLLOW_LOGIND, phase, hold);
    let guest = |phase, hold| runs(IGNORE_LOGIND, phase, hold);
    let off = Locker::default();
    #[rustfmt::skip]
    let cases: Vec<(&str, Locker, LockerEvent, Locker, Vec<LockerEffect>)> = vec![
        // Configuring: the inhibitor is taken only for our own lock in our own session.
        ("own session arms", off, LockerEvent::Configured { own: true, follow_logind: true },
            own(LockPhase::Unlocked, Hold::Armed), vec![TakeInhibitor]),
        ("guest never arms", off, LockerEvent::Configured { own: true, follow_logind: false },
            guest(LockPhase::Unlocked, Hold::Released), vec![]),
        ("reconfigure keeps the hold", own(LockPhase::Unlocked, Hold::Armed),
            LockerEvent::Configured { own: true, follow_logind: true },
            own(LockPhase::Unlocked, Hold::Armed), vec![]),
        ("turning off releases", own(LockPhase::Unlocked, Hold::Armed),
            LockerEvent::Configured { own: false, follow_logind: true },
            Locker { follow_logind: true, ..Locker::default() },
            vec![ReleaseInhibitor]),
        // Asking.
        ("action starts a lock", own(LockPhase::Unlocked, Hold::Armed),
            LockerEvent::Requested(LockCause::Action, T0), own(locking(), Hold::Armed), vec![]),
        ("logind lock starts one in our session", own(LockPhase::Unlocked, Hold::Armed),
            LockerEvent::Requested(LockCause::Logind, T0), own(locking(), Hold::Armed), vec![]),
        ("logind lock is not a guest's", guest(LockPhase::Unlocked, Hold::Released),
            LockerEvent::Requested(LockCause::Logind, T0), guest(LockPhase::Unlocked, Hold::Released), vec![]),
        ("a guest still locks on its own action", guest(LockPhase::Unlocked, Hold::Released),
            LockerEvent::Requested(LockCause::Action, T0), guest(locking(), Hold::Released), vec![]),
        ("never a second lock while locking", own(locking(), Hold::Armed),
            LockerEvent::Requested(LockCause::Action, T0), own(locking(), Hold::Armed), vec![]),
        ("never a second lock while locked", own(LockPhase::Locked, Hold::Armed),
            LockerEvent::Requested(LockCause::Logind, T0), own(LockPhase::Locked, Hold::Armed), vec![]),
        ("off does nothing", off, LockerEvent::Requested(LockCause::Action, T0), off, vec![]),
        // Reports and exits.
        ("report locks", own(locking(), Hold::Armed), LockerEvent::Reported,
            own(LockPhase::Locked, Hold::Armed), vec![]),
        ("report while unlocked is stale", own(LockPhase::Unlocked, Hold::Armed), LockerEvent::Reported,
            own(LockPhase::Unlocked, Hold::Armed), vec![]),
        ("exit unlocks", own(LockPhase::Locked, Hold::Armed), LockerEvent::Exited,
            own(LockPhase::Unlocked, Hold::Armed), vec![]),
        // Sleep.
        ("sleep starts a lock and delays", own(LockPhase::Unlocked, Hold::Armed),
            LockerEvent::SleepStarting(T0), own(locking(), delaying()), vec![]),
        ("sleep while locked lets go at once", own(LockPhase::Locked, Hold::Armed),
            LockerEvent::SleepStarting(T0), own(LockPhase::Locked, Hold::Released), vec![ReleaseInhibitor]),
        ("sleep while locking waits for the report", own(locking(), Hold::Armed),
            LockerEvent::SleepStarting(T0), own(locking(), delaying()), vec![]),
        ("sleep without an inhibitor still locks", own(LockPhase::Unlocked, Hold::Released),
            LockerEvent::SleepStarting(T0), own(locking(), Hold::Released), vec![]),
        ("sleep is not a guest's", guest(LockPhase::Unlocked, Hold::Released),
            LockerEvent::SleepStarting(T0), guest(LockPhase::Unlocked, Hold::Released), vec![]),
        ("the report releases the sleep", own(locking(), delaying()), LockerEvent::Reported,
            own(LockPhase::Locked, Hold::Released), vec![ReleaseInhibitor]),
        ("the lock ending releases the sleep", own(locking(), delaying()), LockerEvent::Exited,
            own(LockPhase::Unlocked, Hold::Released), vec![ReleaseInhibitor]),
        ("the hold runs out", own(locking(), delaying()),
            LockerEvent::Tick(T0.after(SLEEP_HOLD_MS)), own(locking(), Hold::Released),
            vec![ReleaseInhibitor]),
        ("before the hold runs out", own(locking(), delaying()),
            LockerEvent::Tick(T0.after(Ms(SLEEP_HOLD_MS.0 - 1))), own(locking(), delaying()), vec![]),
        ("waking re-arms", own(LockPhase::Locked, Hold::Released), LockerEvent::Woke,
            own(LockPhase::Locked, Hold::Armed), vec![TakeInhibitor]),
        ("waking while armed keeps it", own(LockPhase::Locked, Hold::Armed), LockerEvent::Woke,
            own(LockPhase::Locked, Hold::Armed), vec![]),
        ("a refused inhibitor", own(LockPhase::Unlocked, Hold::Armed), LockerEvent::InhibitorRefused,
            own(LockPhase::Unlocked, Hold::Released), vec![]),
        // A silent lock is taken as locked.
        ("give up waiting for the report", own(locking(), Hold::Armed),
            LockerEvent::Tick(T0.after(READY_GIVE_UP_MS)), own(LockPhase::Locked, Hold::Armed), vec![]),
    ];
    for (name, before, event, want, effects) in cases {
        assert_eq!(step(before, event), (want, effects), "{name}");
    }
}

#[test]
fn deadlines_are_the_earliest_pending() {
    let cases = [
        (runs(FOLLOW_LOGIND, LockPhase::Unlocked, Hold::Armed), None),
        (
            runs(FOLLOW_LOGIND, locking(), Hold::Armed),
            Some(T0.after(READY_GIVE_UP_MS)),
        ),
        (
            runs(FOLLOW_LOGIND, locking(), delaying()),
            Some(T0.after(SLEEP_HOLD_MS)),
        ),
        (
            runs(FOLLOW_LOGIND, LockPhase::Locked, delaying()),
            Some(T0.after(SLEEP_HOLD_MS)),
        ),
    ];
    for (locker, want) in cases {
        assert_eq!(next_deadline(&locker), want, "{locker:?}");
    }
}

#[test]
fn a_whole_sleep_in_order() {
    let events = [
        LockerEvent::Configured {
            own: true,
            follow_logind: true,
        },
        LockerEvent::SleepStarting(T0),
        LockerEvent::Requested(LockCause::Logind, T0.after(Ms(10))),
        LockerEvent::Reported,
        LockerEvent::Woke,
        LockerEvent::Exited,
    ];
    let (last, effects) = events.into_iter().fold(
        (Locker::default(), Vec::new()),
        |(locker, mut all), event| {
            let (next, effects) = step(locker, event);
            all.extend(effects);
            (next, all)
        },
    );
    assert_eq!(
        effects,
        vec![TakeInhibitor, ReleaseInhibitor, TakeInhibitor]
    );
    assert_eq!(last, runs(FOLLOW_LOGIND, LockPhase::Unlocked, Hold::Armed));
}
