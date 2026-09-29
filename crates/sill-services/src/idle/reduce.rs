//! The idle machine: the one place idleness changes. The live side turns idle notifications,
//! logind's signals, the lock, the battery and the inhibitors into [`IdleEvent`]s and performs the
//! [`IdleEffect`]s, in order.
//!
//! The rules it implements (each a row of `reduce_tests.rs`):
//! - `Policy` / `Power` / `Inhibition` / `Lock` changes re-derive [`super::watches`] and emit
//!   `Watch` when they differ from the held set (`Watch(vec![])` while `Off` or inhibited).
//!   A policy that turns the mode `Off` also undoes whatever idle had done (undim, displays on).
//! - `Idled(Dim)` while `Active`: `Dim(level)`, phase `Dimmed`.
//! - `Idled(ScreenOff)`: with `IdleLock::AtScreenOff` and the session unlocked, `Lock` first,
//!   then `Undim`, `Displays(Off)` and `IdleHint(Idle)`; the lock's instant is kept for the
//!   grace. Already locked (someone locked earlier): no `Lock`, and no grace later.
//! - `Idled(LockedScreenOff)` while locked: `Displays(Off)` (no new lock, the instant kept).
//! - `Idled(Suspend)`: `Suspend`, phase `Suspending`.
//! - `Resumed` (real input): `Undim` if dimmed; if the displays were off, `Displays(On)`,
//!   `IdleHint(Active)` and, when this idle lock is still in place, `Grace(case)` with the case
//!   [`super::grace_verdict`] reads (the live side mints the ticket; `Ask` sends nothing).
//! - `SleepStarting`: marks `SleepSince::Slept` for the lock in place. `Woke`: `Displays(On)`,
//!   phase `Active`, the watches held again.
//! - `Lock` seen `Unlocked`: the kept instant and the sleep mark are forgotten.
//! - Every event while the mode is `Off` changes nothing and emits nothing (the passive start).

use sill_model::locker::LockCause;
use sill_model::units::Stamp;

use super::grace::{GraceCase, SleepSince};
use super::inhibit::Inhibition;
use super::model::{DimShown, IdlePhase};
use super::policy::DimPlan;
use super::policy::{
    DimLevel, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace, LockView,
    LockedScreenOff, PowerPair, PowerSource, Threshold, Timeout, Watch,
};

/// The machine's whole state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdleState {
    pub policy: IdlePolicy,
    pub power: PowerSource,
    pub inhibition: Inhibition,
    pub phase: IdlePhase,
    pub dim: DimShown,
    pub lock: LockSeen,
    /// When this idle path locked, while that lock is in place.
    pub idle_locked_at: Option<Stamp>,
    pub sleep: SleepSince,
    /// The notifications held now.
    pub held: Vec<Watch>,
}

impl Default for IdleState {
    fn default() -> Self {
        IdleState {
            policy: PASSIVE,
            power: PowerSource::Ac,
            inhibition: Inhibition::Free,
            phase: IdlePhase::Active,
            dim: DimShown::Clear,
            lock: LockSeen::UNLOCKED,
            idle_locked_at: None,
            sleep: SleepSince::NoSleep,
            held: Vec::new(),
        }
    }
}

/// The policy before the daemon's first: mode `Off`, and times that are never used.
pub const PASSIVE: IdlePolicy = IdlePolicy {
    mode: IdleMode::Off(super::policy::OffReason::NoPolicy),
    times: IdleTimes {
        screen_off: PowerPair {
            ac: Timeout::Never,
            battery: Timeout::Never,
        },
        suspend: PowerPair {
            ac: Timeout::Never,
            battery: Timeout::Never,
        },
    },
    dim: DimPlan {
        lead: IdleSecs(0),
        level: DimLevel(100),
    },
    lock: IdleLock::Never,
    locked_off: LockedScreenOff(IdleSecs(0)),
    grace: LockGrace(IdleSecs(0)),
};

/// The lock as idle sees it (from the locker's published `Lock`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockSeen {
    pub view: LockView,
    pub cause: Option<LockCause>,
}

impl LockSeen {
    pub const UNLOCKED: LockSeen = LockSeen {
        view: LockView::Unlocked,
        cause: None,
    };
}

/// One thing the live side learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleEvent {
    Policy(IdlePolicy),
    Power(PowerSource),
    Inhibition(Inhibition),
    /// `ext_idle_notification_v1.idled` for the watch of that threshold.
    Idled(Threshold, Stamp),
    /// `ext_idle_notification_v1.resumed`: real input.
    Resumed(Stamp),
    Lock(LockSeen),
    /// logind's `PrepareForSleep(true)`.
    SleepStarting(Stamp),
    /// logind's `PrepareForSleep(false)`.
    Woke(Stamp),
}

/// Output power, through `zwlr_output_power_manager_v1` on every output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayPower {
    Off,
    On,
}

/// logind's `Session.SetIdleHint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleHint {
    Idle,
    Active,
}

/// What the live side must do, in order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IdleEffect {
    /// Destroy every idle notification and hold these instead.
    Watch(Vec<Watch>),
    /// Publish `DimShown::Dimmed(level)` (the overlay fades in with quire's motion).
    Dim(DimLevel),
    Undim,
    Displays(DisplayPower),
    /// `LockerCommand::IdleLock`.
    Lock,
    /// Run the grace rule on this case; on `NoPassword`, `AuthCommand::Grace(ticket)`.
    Grace(GraceCase),
    /// logind's `Suspend(false)` (never `interactive`).
    Suspend,
    IdleHint(IdleHint),
}

/// `state` after `event`, and the effects to perform (module doc).
pub fn step(state: IdleState, event: IdleEvent) -> (IdleState, Vec<IdleEffect>) {
    rules::step(state, event)
}

#[path = "reduce_rules.rs"]
mod rules;

#[cfg(test)]
#[path = "reduce_tests.rs"]
mod tests;
