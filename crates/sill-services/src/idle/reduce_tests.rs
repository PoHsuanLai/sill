use super::{DisplayPower, IdleEffect, IdleEvent, IdleHint, IdleState, LockSeen, step};
use crate::idle::grace::{GraceCase, SleepSince, WakeDelay};
use crate::idle::model::{DimShown, IdlePhase};
use crate::idle::policy::{
    DimLevel, DimPlan, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace, LockView,
    LockedScreenOff, OffReason, PowerPair, Threshold, Timeout,
};
use sill_model::locker::LockCause;
use sill_model::units::Stamp;

fn own() -> IdlePolicy {
    let after = |s| Timeout::After(IdleSecs(s));
    IdlePolicy {
        mode: IdleMode::Own,
        times: IdleTimes {
            screen_off: PowerPair {
                ac: after(900),
                battery: after(900),
            },
            suspend: PowerPair {
                ac: after(1_800),
                battery: after(900),
            },
        },
        dim: DimPlan {
            lead: IdleSecs(5),
            level: DimLevel(50),
        },
        lock: IdleLock::AtScreenOff,
        locked_off: LockedScreenOff(IdleSecs(60)),
        grace: LockGrace(IdleSecs(5)),
    }
}

fn idle_locked() -> LockSeen {
    LockSeen {
        view: LockView::Locked,
        cause: Some(LockCause::IdleScreenOff),
    }
}

/// Runs `events` from the default state and returns the last state and every effect.
fn run(events: &[IdleEvent]) -> (IdleState, Vec<IdleEffect>) {
    let mut state = IdleState::default();
    let mut all = Vec::new();
    for event in events {
        let (next, effects) = step(state, *event);
        state = next;
        all.extend(effects);
    }
    (state, all)
}

#[test]
fn passive_until_the_first_policy() {
    let (state, effects) = run(&[
        IdleEvent::Idled(Threshold::ScreenOff, Stamp(1)),
        IdleEvent::Resumed(Stamp(2)),
        IdleEvent::SleepStarting(Stamp(3)),
    ]);
    assert!(effects.is_empty(), "{effects:?}");
    assert_eq!(state.policy.mode, IdleMode::Off(OffReason::NoPolicy));
}

#[test]
fn dim_then_off_locks_first_and_a_quick_wake_earns_the_grace() {
    let (state, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::Dim, Stamp(895_000)),
        IdleEvent::Idled(Threshold::ScreenOff, Stamp(900_000)),
        IdleEvent::Lock(idle_locked()),
        IdleEvent::Resumed(Stamp(903_000)),
    ]);
    let tail: Vec<IdleEffect> = effects
        .into_iter()
        .filter(|e| !matches!(e, IdleEffect::Watch(_)))
        .collect();
    assert_eq!(
        tail,
        vec![
            IdleEffect::Dim(DimLevel(50)),
            IdleEffect::Lock,
            IdleEffect::Undim,
            IdleEffect::Displays(DisplayPower::Off),
            IdleEffect::IdleHint(IdleHint::Idle),
            IdleEffect::Displays(DisplayPower::On),
            IdleEffect::IdleHint(IdleHint::Active),
            IdleEffect::Grace(GraceCase {
                cause: Some(LockCause::IdleScreenOff),
                grace: LockGrace(IdleSecs(5)),
                woke: WakeDelay(3_000),
                sleep: SleepSince::NoSleep,
            }),
        ]
    );
    assert_eq!(state.phase, IdlePhase::Active);
    assert_eq!(state.dim, DimShown::Clear);
}

#[test]
fn input_during_the_dim_undims_and_nothing_locks() {
    let (state, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::Dim, Stamp(10)),
        IdleEvent::Resumed(Stamp(20)),
    ]);
    assert!(effects.contains(&IdleEffect::Undim));
    assert!(!effects.contains(&IdleEffect::Lock));
    assert_eq!(state.phase, IdlePhase::Active);
}

#[test]
fn an_earlier_lock_keeps_its_cause_and_gets_no_grace() {
    let asked = LockSeen {
        cause: Some(LockCause::Action),
        ..idle_locked()
    };
    let (_, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Lock(asked),
        IdleEvent::Idled(Threshold::LockedScreenOff, Stamp(60_000)),
        IdleEvent::Resumed(Stamp(61_000)),
    ]);
    assert!(!effects.contains(&IdleEffect::Lock), "no second lock");
    assert!(
        !effects.iter().any(|e| matches!(e, IdleEffect::Grace(_))),
        "{effects:?}"
    );
}

#[test]
fn a_sleep_after_the_idle_lock_marks_the_case() {
    let (_, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::ScreenOff, Stamp(900_000)),
        IdleEvent::Lock(idle_locked()),
        IdleEvent::Idled(Threshold::Suspend, Stamp(1_800_000)),
        IdleEvent::SleepStarting(Stamp(1_800_010)),
        IdleEvent::Woke(Stamp(1_800_020)),
        IdleEvent::Resumed(Stamp(1_800_030)),
    ]);
    assert!(effects.contains(&IdleEffect::Suspend));
    let graces: Vec<&IdleEffect> = effects
        .iter()
        .filter(|e| matches!(e, IdleEffect::Grace(_)))
        .collect();
    assert!(
        graces
            .iter()
            .all(|e| matches!(e, IdleEffect::Grace(case) if case.sleep == SleepSince::Slept)),
        "{graces:?}"
    );
}

#[test]
fn turning_off_undoes_what_idle_did() {
    let (state, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::Dim, Stamp(10)),
        IdleEvent::Policy(IdlePolicy {
            mode: IdleMode::Off(OffReason::CosmicIdleRunning),
            ..own()
        }),
    ]);
    assert!(effects.contains(&IdleEffect::Undim));
    assert!(effects.contains(&IdleEffect::Watch(Vec::new())));
    assert_eq!(state.phase, IdlePhase::Active);
}

#[test]
fn an_inhibitor_clears_the_dim_and_drops_the_watches() {
    use crate::idle::inhibit::Inhibition;
    let (state, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::Dim, Stamp(10)),
        IdleEvent::Inhibition(Inhibition::Held),
    ]);
    let tail = &effects[effects.len() - 2..];
    assert_eq!(tail, [IdleEffect::Undim, IdleEffect::Watch(Vec::new())]);
    assert_eq!(
        (state.phase, state.dim),
        (IdlePhase::Active, DimShown::Clear)
    );
    let (_, again) = step(state, IdleEvent::Inhibition(Inhibition::Free));
    assert!(
        matches!(again.as_slice(), [IdleEffect::Watch(w)] if w.len() == 3),
        "{again:?}"
    );
}

#[test]
fn the_idle_lock_keeps_the_suspend_watch_as_it_was() {
    use crate::idle::policy::Watch;
    let (_, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::ScreenOff, Stamp(900_000)),
        IdleEvent::Lock(idle_locked()),
    ]);
    let sets: Vec<&Vec<Watch>> = effects
        .iter()
        .filter_map(|e| match e {
            IdleEffect::Watch(set) => Some(set),
            _ => None,
        })
        .collect();
    let suspend = |set: &Vec<Watch>| set.iter().find(|w| w.stage == Threshold::Suspend).copied();
    assert_eq!(sets.len(), 2, "{sets:?}");
    assert_eq!(suspend(sets[0]), suspend(sets[1]), "G342");
    assert!(
        sets[1]
            .iter()
            .any(|w| w.stage == Threshold::LockedScreenOff)
    );
}

#[test]
fn waking_from_sleep_holds_every_watch_afresh() {
    let (state, effects) = run(&[
        IdleEvent::Policy(own()),
        IdleEvent::Idled(Threshold::ScreenOff, Stamp(900_000)),
        IdleEvent::Idled(Threshold::Suspend, Stamp(1_800_000)),
        IdleEvent::SleepStarting(Stamp(1_800_010)),
    ]);
    let held = state.held.clone();
    let (after, woke) = step(state, IdleEvent::Woke(Stamp(1_900_000)));
    assert_eq!(
        woke,
        vec![
            IdleEffect::Displays(DisplayPower::On),
            IdleEffect::IdleHint(IdleHint::Active),
            IdleEffect::Watch(Vec::new()),
            IdleEffect::Watch(held),
        ]
    );
    assert_eq!(after.phase, IdlePhase::Active);
    assert!(effects.contains(&IdleEffect::Suspend));
}
