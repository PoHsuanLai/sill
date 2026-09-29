//! The locker's pure state machine: when to lock, when to hold and release logind's sleep
//! delay, and what the lock screen is doing. [`step`] is the only place it changes; the
//! live side performs the [`LockerEffect`]s it returns and feeds back what happened.
//!
//! The sleep delay follows logind's rule that a delay inhibitor only delays a sleep it was
//! already held for: it is taken as soon as the lock is configured (and again after every
//! resume), and released once the lock reports locked, the lock ends, or
//! [`SLEEP_HOLD_MS`] runs out, whichever is first.

use serde::{Deserialize, Serialize};

use sill_model::units::Stamp;

/// How long the lock may take to report locked before it is taken as locked anyway.
pub const READY_GIVE_UP_MS: u64 = 10_000;

/// How long sleep waits for the lock at most: under logind's default `InhibitDelayMaxSec` (5 s),
/// so it is sill, not logind, that lets go.
pub const SLEEP_HOLD_MS: u64 = 4_000;

/// What the lock screen is doing, as surfaces and tests see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockPhase {
    #[default]
    Unlocked,
    /// A lock was asked for and has not said locked.
    Locking,
    Locked,
}

/// Who carries out a lock, once the setting is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Runner {
    /// Nothing locks.
    #[default]
    Off,
    /// sill's own lock screen.
    Own,
}

/// Whether logind's `Lock` and `PrepareForSleep` start a lock, or only sill's own Lock does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Triggers {
    /// A sill session: logind's signals are ours to act on.
    FollowLogind,
    /// Another desktop owns this logind session (sill nested during development, or
    /// cosmic-session): its signals are not ours.
    #[default]
    IgnoreLogind,
}

/// Why a lock was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cause {
    /// sill's Lock: the launcher row, the power menu, `sill lock`.
    Action,
    /// logind's `Lock` signal for this session (`loginctl lock-session`, Super+Escape).
    Logind,
    /// sill's own idle service turned the displays off.
    Idle,
}

/// The sleep delay inhibitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Hold {
    /// None held.
    #[default]
    Released,
    /// Held, waiting for a sleep.
    Armed,
    /// A sleep is waiting on it until the lock reports locked or `until`.
    Delaying { until: Stamp },
}

/// The locker service's whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Locker {
    pub runner: Runner,
    pub triggers: Triggers,
    pub phase: Phase,
    pub hold: Hold,
}

/// [`LockPhase`] with the deadline the machine needs while locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    #[default]
    Unlocked,
    Locking {
        give_up: Stamp,
    },
    Locked,
}

impl Phase {
    /// The published form.
    pub fn public(self) -> LockPhase {
        match self {
            Phase::Unlocked => LockPhase::Unlocked,
            Phase::Locking { .. } => LockPhase::Locking,
            Phase::Locked => LockPhase::Locked,
        }
    }
}

/// One thing the live side learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockerEvent {
    /// The mode resolved.
    Configured(Runner, Triggers),
    /// A lock was asked for.
    Requested(Cause, Stamp),
    /// logind's `PrepareForSleep(true)`.
    SleepStarting(Stamp),
    /// logind's `PrepareForSleep(false)`.
    Woke,
    /// The compositor granted the lock.
    Reported,
    /// The lock ended, or could not be taken.
    Exited,
    /// Taking the inhibitor failed.
    InhibitorRefused,
    /// Time passed.
    Tick(Stamp),
}

/// What the live side must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockerEffect {
    /// Take `Inhibit("sleep", "sill", "lock before sleep", "delay")`.
    TakeInhibitor,
    /// Close the inhibitor's descriptor.
    ReleaseInhibitor,
}

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

#[cfg(test)]
#[path = "machine_tests.rs"]
mod tests;
