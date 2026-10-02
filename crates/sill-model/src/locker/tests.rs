use LockerEffect::{ReleaseInhibitor, TakeInhibitor};
use Triggers::{FollowLogind, IgnoreLogind};

use super::{
    Cause, Hold, Locker, LockerEffect, LockerEvent, Phase, READY_GIVE_UP_MS, Runner, SLEEP_HOLD_MS,
    Triggers, next_deadline, step,
};
use ds_core::time::stamp::Stamp;

const T0: Stamp = Stamp(1_000);

fn runs(triggers: Triggers, phase: Phase, hold: Hold) -> Locker {
    Locker {
        runner: Runner::Own,
        triggers,
        phase,
        hold,
    }
}

fn locking() -> Phase {
    Phase::Locking {
        give_up: T0.after(READY_GIVE_UP_MS),
    }
}

fn delaying() -> Hold {
    Hold::Delaying {
        until: T0.after(SLEEP_HOLD_MS),
    }
}

#[test]
fn every_transition() {
    let own = |phase, hold| runs(FollowLogind, phase, hold);
    let guest = |phase, hold| runs(IgnoreLogind, phase, hold);
    let off = Locker::default();
    #[rustfmt::skip]
    let cases: Vec<(&str, Locker, LockerEvent, Locker, Vec<LockerEffect>)> = vec![
        // Configuring: the inhibitor is taken only for our own lock in our own session.
        ("own session arms", off, LockerEvent::Configured(Runner::Own, FollowLogind),
            own(Phase::Unlocked, Hold::Armed), vec![TakeInhibitor]),
        ("guest never arms", off, LockerEvent::Configured(Runner::Own, IgnoreLogind),
            guest(Phase::Unlocked, Hold::Released), vec![]),
        ("reconfigure keeps the hold", own(Phase::Unlocked, Hold::Armed),
            LockerEvent::Configured(Runner::Own, FollowLogind),
            own(Phase::Unlocked, Hold::Armed), vec![]),
        ("turning off releases", own(Phase::Unlocked, Hold::Armed),
            LockerEvent::Configured(Runner::Off, FollowLogind),
            Locker { runner: Runner::Off, triggers: FollowLogind, ..Locker::default() },
            vec![ReleaseInhibitor]),
        // Asking.
        ("action starts a lock", own(Phase::Unlocked, Hold::Armed),
            LockerEvent::Requested(Cause::Action, T0), own(locking(), Hold::Armed), vec![]),
        ("logind lock starts one in our session", own(Phase::Unlocked, Hold::Armed),
            LockerEvent::Requested(Cause::Logind, T0), own(locking(), Hold::Armed), vec![]),
        ("logind lock is not a guest's", guest(Phase::Unlocked, Hold::Released),
            LockerEvent::Requested(Cause::Logind, T0), guest(Phase::Unlocked, Hold::Released), vec![]),
        ("a guest still locks on its own action", guest(Phase::Unlocked, Hold::Released),
            LockerEvent::Requested(Cause::Action, T0), guest(locking(), Hold::Released), vec![]),
        ("never a second lock while locking", own(locking(), Hold::Armed),
            LockerEvent::Requested(Cause::Action, T0), own(locking(), Hold::Armed), vec![]),
        ("never a second lock while locked", own(Phase::Locked, Hold::Armed),
            LockerEvent::Requested(Cause::Logind, T0), own(Phase::Locked, Hold::Armed), vec![]),
        ("off does nothing", off, LockerEvent::Requested(Cause::Action, T0), off, vec![]),
        // Reports and exits.
        ("report locks", own(locking(), Hold::Armed), LockerEvent::Reported,
            own(Phase::Locked, Hold::Armed), vec![]),
        ("report while unlocked is stale", own(Phase::Unlocked, Hold::Armed), LockerEvent::Reported,
            own(Phase::Unlocked, Hold::Armed), vec![]),
        ("exit unlocks", own(Phase::Locked, Hold::Armed), LockerEvent::Exited,
            own(Phase::Unlocked, Hold::Armed), vec![]),
        // Sleep.
        ("sleep starts a lock and delays", own(Phase::Unlocked, Hold::Armed),
            LockerEvent::SleepStarting(T0), own(locking(), delaying()), vec![]),
        ("sleep while locked lets go at once", own(Phase::Locked, Hold::Armed),
            LockerEvent::SleepStarting(T0), own(Phase::Locked, Hold::Released), vec![ReleaseInhibitor]),
        ("sleep while locking waits for the report", own(locking(), Hold::Armed),
            LockerEvent::SleepStarting(T0), own(locking(), delaying()), vec![]),
        ("sleep without an inhibitor still locks", own(Phase::Unlocked, Hold::Released),
            LockerEvent::SleepStarting(T0), own(locking(), Hold::Released), vec![]),
        ("sleep is not a guest's", guest(Phase::Unlocked, Hold::Released),
            LockerEvent::SleepStarting(T0), guest(Phase::Unlocked, Hold::Released), vec![]),
        ("the report releases the sleep", own(locking(), delaying()), LockerEvent::Reported,
            own(Phase::Locked, Hold::Released), vec![ReleaseInhibitor]),
        ("the lock ending releases the sleep", own(locking(), delaying()), LockerEvent::Exited,
            own(Phase::Unlocked, Hold::Released), vec![ReleaseInhibitor]),
        ("the hold runs out", own(locking(), delaying()),
            LockerEvent::Tick(T0.after(SLEEP_HOLD_MS)), own(locking(), Hold::Released),
            vec![ReleaseInhibitor]),
        ("before the hold runs out", own(locking(), delaying()),
            LockerEvent::Tick(T0.after(SLEEP_HOLD_MS - 1)), own(locking(), delaying()), vec![]),
        ("waking re-arms", own(Phase::Locked, Hold::Released), LockerEvent::Woke,
            own(Phase::Locked, Hold::Armed), vec![TakeInhibitor]),
        ("waking while armed keeps it", own(Phase::Locked, Hold::Armed), LockerEvent::Woke,
            own(Phase::Locked, Hold::Armed), vec![]),
        ("a refused inhibitor", own(Phase::Unlocked, Hold::Armed), LockerEvent::InhibitorRefused,
            own(Phase::Unlocked, Hold::Released), vec![]),
        // A silent lock is taken as locked.
        ("give up waiting for the report", own(locking(), Hold::Armed),
            LockerEvent::Tick(T0.after(READY_GIVE_UP_MS)), own(Phase::Locked, Hold::Armed), vec![]),
    ];
    for (name, before, event, want, effects) in cases {
        assert_eq!(step(before, event), (want, effects), "{name}");
    }
}

#[test]
fn deadlines_are_the_earliest_pending() {
    let cases = [
        (runs(FollowLogind, Phase::Unlocked, Hold::Armed), None),
        (
            runs(FollowLogind, locking(), Hold::Armed),
            Some(T0.after(READY_GIVE_UP_MS)),
        ),
        (
            runs(FollowLogind, locking(), delaying()),
            Some(T0.after(SLEEP_HOLD_MS)),
        ),
        (
            runs(FollowLogind, Phase::Locked, delaying()),
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
        LockerEvent::Configured(Runner::Own, FollowLogind),
        LockerEvent::SleepStarting(T0),
        LockerEvent::Requested(Cause::Logind, T0.after(10)),
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
    assert_eq!(last, runs(FollowLogind, Phase::Unlocked, Hold::Armed));
}
