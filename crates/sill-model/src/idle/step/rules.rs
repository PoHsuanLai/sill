//! The machine's rules, one function per event (the module doc of `reduce.rs` lists them).

use super::super::grace::{GraceCase, SleepSince, WakeDelay};
use super::super::inhibit::Inhibition;
use super::super::model::{DimShown, IdlePhase};
use super::super::policy::{IdleLock, IdleMode, IdlePolicy, LockView, Threshold, Watch, watches};
use super::{DisplayPower, IdleEffect, IdleEvent, IdleHint, IdleState, LockSeen};
use crate::units::Stamp;

type Step = (IdleState, Vec<IdleEffect>);

pub(super) fn step(state: IdleState, event: IdleEvent) -> Step {
    match (state.policy.mode, event) {
        (_, IdleEvent::Policy(policy)) => policy_changed(state, policy),
        (IdleMode::Off(_), other) => quietly(state, other),
        (IdleMode::Own, IdleEvent::Power(power)) => rewatch(IdleState { power, ..state }),
        (IdleMode::Own, IdleEvent::Inhibition(inhibition)) => inhibited(state, inhibition),
        (IdleMode::Own, IdleEvent::Lock(seen)) => lock_seen(state, seen),
        (IdleMode::Own, IdleEvent::Idled(stage, now)) => idled(state, stage, now),
        (IdleMode::Own, IdleEvent::Resumed(now)) => resumed(state, now),
        (IdleMode::Own, IdleEvent::SleepStarting(_)) => (
            IdleState {
                sleep: SleepSince::Slept,
                ..state
            },
            Vec::new(),
        ),
        (IdleMode::Own, IdleEvent::Woke(_)) => woke(state),
    }
}

/// While `Off`, the facts idle follows are kept (so turning on starts from the truth) and
/// nothing is done.
fn quietly(state: IdleState, event: IdleEvent) -> Step {
    let next = match event {
        IdleEvent::Power(power) => IdleState { power, ..state },
        IdleEvent::Inhibition(inhibition) => IdleState {
            inhibition,
            ..state
        },
        IdleEvent::Lock(seen) => forget_if_unlocked(IdleState {
            lock: seen,
            ..state
        }),
        IdleEvent::Policy(_)
        | IdleEvent::Idled(..)
        | IdleEvent::Resumed(_)
        | IdleEvent::SleepStarting(_)
        | IdleEvent::Woke(_) => state,
    };
    (next, Vec::new())
}

fn policy_changed(state: IdleState, policy: IdlePolicy) -> Step {
    match policy.mode {
        IdleMode::Own => rewatch(IdleState { policy, ..state }),
        IdleMode::Off(_) => {
            let (undone, mut effects) = undo(IdleState { policy, ..state });
            if !undone.held.is_empty() {
                effects.push(IdleEffect::Watch(Vec::new()));
            }
            (
                IdleState {
                    held: Vec::new(),
                    idle_locked_at: None,
                    ..undone
                },
                effects,
            )
        }
    }
}

/// Undim, and turn the displays back on if idle had turned them off.
fn undo(state: IdleState) -> Step {
    let mut effects = undim(&state);
    match state.phase {
        IdlePhase::DisplaysOff | IdlePhase::Suspending => {
            effects.push(IdleEffect::Displays(DisplayPower::On));
            effects.push(IdleEffect::IdleHint(IdleHint::Active));
        }
        IdlePhase::Active | IdlePhase::Dimmed => {}
    }
    (
        IdleState {
            phase: IdlePhase::Active,
            dim: DimShown::Clear,
            ..state
        },
        effects,
    )
}

fn undim(state: &IdleState) -> Vec<IdleEffect> {
    match state.dim {
        DimShown::Dimmed(_) => vec![IdleEffect::Undim],
        DimShown::Clear => Vec::new(),
    }
}

/// The notifications `state` wants now.
fn wanted(state: &IdleState) -> Vec<Watch> {
    match state.inhibition {
        Inhibition::Held => Vec::new(),
        Inhibition::Free => watches(&state.policy, state.power, state.lock.view),
    }
}

/// Emit `Watch` when the wanted set differs from the held one.
fn rewatch(state: IdleState) -> Step {
    let want = wanted(&state);
    if want == state.held {
        return (state, Vec::new());
    }
    (
        IdleState {
            held: want.clone(),
            ..state
        },
        vec![IdleEffect::Watch(want)],
    )
}

/// An inhibitor taken while dimmed clears the dim at once: its notifications go, so no
/// `resumed` would ever clear it.
fn inhibited(state: IdleState, inhibition: Inhibition) -> Step {
    let state = IdleState {
        inhibition,
        ..state
    };
    let (state, mut effects) = match (inhibition, state.phase) {
        (Inhibition::Held, IdlePhase::Dimmed) => (
            IdleState {
                phase: IdlePhase::Active,
                dim: DimShown::Clear,
                ..state
            },
            vec![IdleEffect::Undim],
        ),
        (Inhibition::Held | Inhibition::Free, _) => (state, Vec::new()),
    };
    let (state, watch) = rewatch(state);
    effects.extend(watch);
    (state, effects)
}

fn lock_seen(state: IdleState, seen: LockSeen) -> Step {
    rewatch(forget_if_unlocked(IdleState {
        lock: seen,
        ..state
    }))
}

/// Unlocked: the idle lock's instant and the sleep mark belong to a lock that is gone.
fn forget_if_unlocked(state: IdleState) -> IdleState {
    match state.lock.view {
        LockView::Unlocked => IdleState {
            idle_locked_at: None,
            sleep: SleepSince::NoSleep,
            ..state
        },
        LockView::Locked => state,
    }
}

fn idled(state: IdleState, stage: Threshold, now: Stamp) -> Step {
    match (stage, state.phase) {
        (Threshold::Dim, IdlePhase::Active) => {
            let level = state.policy.dim.level;
            (
                IdleState {
                    phase: IdlePhase::Dimmed,
                    dim: DimShown::Dimmed(level),
                    ..state
                },
                vec![IdleEffect::Dim(level)],
            )
        }
        (Threshold::ScreenOff, IdlePhase::Active | IdlePhase::Dimmed) => screen_off(state, now),
        (Threshold::LockedScreenOff, IdlePhase::Active | IdlePhase::Dimmed) => {
            match state.lock.view {
                LockView::Locked => displays_off(state, Vec::new()),
                LockView::Unlocked => (state, Vec::new()),
            }
        }
        (Threshold::Suspend, IdlePhase::Active | IdlePhase::Dimmed | IdlePhase::DisplaysOff) => (
            IdleState {
                phase: IdlePhase::Suspending,
                ..state
            },
            vec![IdleEffect::Suspend],
        ),
        (Threshold::Dim, IdlePhase::Dimmed | IdlePhase::DisplaysOff | IdlePhase::Suspending)
        | (
            Threshold::ScreenOff | Threshold::LockedScreenOff,
            IdlePhase::DisplaysOff | IdlePhase::Suspending,
        )
        | (Threshold::Suspend, IdlePhase::Suspending) => (state, Vec::new()),
    }
}

/// The idle screen-off: lock first (only an unlocked session, only with `AtScreenOff`), then
/// the displays.
fn screen_off(state: IdleState, now: Stamp) -> Step {
    match (state.policy.lock, state.lock.view) {
        (IdleLock::AtScreenOff, LockView::Unlocked) => displays_off(
            IdleState {
                idle_locked_at: Some(now),
                sleep: SleepSince::NoSleep,
                ..state
            },
            vec![IdleEffect::Lock],
        ),
        (IdleLock::AtScreenOff, LockView::Locked) | (IdleLock::Never, _) => {
            displays_off(state, Vec::new())
        }
    }
}

fn displays_off(state: IdleState, mut effects: Vec<IdleEffect>) -> Step {
    effects.extend(undim(&state));
    effects.push(IdleEffect::Displays(DisplayPower::Off));
    effects.push(IdleEffect::IdleHint(IdleHint::Idle));
    (
        IdleState {
            phase: IdlePhase::DisplaysOff,
            dim: DimShown::Clear,
            ..state
        },
        effects,
    )
}

/// Real input: undim; if the displays were off, on again and the grace case for the idle lock
/// still in place.
fn resumed(state: IdleState, now: Stamp) -> Step {
    let was_off = match state.phase {
        IdlePhase::DisplaysOff | IdlePhase::Suspending => true,
        IdlePhase::Active | IdlePhase::Dimmed => false,
    };
    let (state, mut effects) = undo(state);
    if !was_off {
        return (state, effects);
    }
    if let (Some(at), LockView::Locked) = (state.idle_locked_at, state.lock.view) {
        effects.push(IdleEffect::Grace(GraceCase {
            cause: state.lock.cause,
            grace: state.policy.grace,
            woke: WakeDelay(now.since(at)),
            sleep: state.sleep,
        }));
    }
    (state, effects)
}

/// A resume from sleep: displays on, and every notification held afresh (their timers count
/// from now; a seat already idle would otherwise never fire again).
fn woke(state: IdleState) -> Step {
    let (state, mut effects) = undo(state);
    if !effects.contains(&IdleEffect::Displays(DisplayPower::On)) {
        effects.push(IdleEffect::Displays(DisplayPower::On));
    }
    let want = wanted(&state);
    if !state.held.is_empty() {
        effects.push(IdleEffect::Watch(Vec::new()));
    }
    effects.push(IdleEffect::Watch(want.clone()));
    (
        IdleState {
            held: want,
            ..state
        },
        effects,
    )
}
