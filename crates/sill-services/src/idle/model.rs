//! The idle service's published state, commands, wiring and set-up.

use std::path::PathBuf;

use tokio::sync::watch;

use crate::auth::AuthCommand;
use crate::battery::Battery;
use crate::control::Control;
use crate::env::{Bus, Env};
use crate::locker::{Lock, LockerCommand};

use super::inhibit::{Inhibition, LogindIdle};
use super::policy::{DimLevel, IdleConfig, IdleMode, IdlePolicy, PowerSource};

/// What surfaces (the dim overlay) and `sill idle status` read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Idle {
    pub mode: IdleMode,
    pub phase: IdlePhase,
    /// The dim overlay's wish: every output's dim layer shows `Dimmed`.
    pub dim: DimShown,
    pub power: PowerSource,
    pub inhibition: Inhibition,
    /// `org.freedesktop.ScreenSaver` inhibits held (for `sill idle status`).
    pub inhibits: u32,
    /// Whether logind's `BlockInhibited` names `idle`.
    pub logind: LogindIdle,
}

/// Where idleness has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IdlePhase {
    /// In use, or idle below the first threshold.
    #[default]
    Active,
    /// Dimmed, the warning before the displays go off.
    Dimmed,
    /// The displays are off (locked or not).
    DisplaysOff,
    /// sill asked logind to suspend and no wake has come yet.
    Suspending,
}

/// The dim overlay's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DimShown {
    #[default]
    Clear,
    Dimmed(DimLevel),
}

/// What the idle service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleCommand {
    /// The settings (at start and on every change), resolved by the service against this
    /// session (cosmic-session, a guest, a cosmic-idle running) and COSMIC's config. The service
    /// is passive until the first one.
    Configure(IdleConfig),
    /// An already resolved policy (live tests on the nested compositor).
    Use(IdlePolicy),
    /// A made-up `resumed`, then every notification held afresh (`sill debug idle-wake`, sent
    /// only by a daemon built with `debug`): the nested cosmic-comp counts no synthetic input
    /// as activity.
    FakeWake,
}

/// The other services idle reads and drives.
#[derive(Debug, Clone)]
pub struct IdleLinks {
    /// The lock in place and its cause (the grace, and the lock screen's own screen-off time).
    pub lock: watch::Receiver<Lock>,
    /// AC or battery.
    pub battery: watch::Receiver<Battery>,
    /// `LockerCommand::IdleLock` at an idle screen-off.
    pub locker: Control<LockerCommand>,
    /// `AuthCommand::Grace` when the grace opens the lock.
    pub auth: Control<AuthCommand>,
}

/// Where the service connects. The default is the session's own; live tests name a nested
/// compositor and private buses so nothing reaches the real displays, logind or the session
/// bus's `org.freedesktop.ScreenSaver`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub display: IdleDisplay,
    /// logind (`Suspend`, `SetIdleHint`, `PrepareForSleep`, `BlockInhibited`).
    pub system: Bus,
    /// `org.freedesktop.ScreenSaver`.
    pub session: Bus,
}

impl Setup {
    /// The session's compositor, and `env`'s buses.
    pub fn of(env: &Env) -> Setup {
        Setup {
            display: IdleDisplay::Environment,
            system: env.buses.system.clone(),
            session: env.buses.session.clone(),
        }
    }
}

/// The compositor to hold idle notifications and output power on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IdleDisplay {
    /// `$WAYLAND_DISPLAY`.
    #[default]
    Environment,
    /// A socket path (a live test's nested cosmic-comp).
    Socket(PathBuf),
}
