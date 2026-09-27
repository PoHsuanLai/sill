//! The idle service's published state, commands, wiring and set-up (sill FINDINGS "sill idle
//! (Q420 B)").

use std::path::PathBuf;

use tokio::sync::watch;

use crate::auth::AuthCommand;
use crate::battery::Battery;
use crate::control::Control;
use crate::locker::{Lock, LockerCommand};

use super::inhibit::{Inhibition, LogindIdle};
use super::policy::{DimLevel, IdleConfig, IdleMode, IdlePolicy, PowerSource};

/// What surfaces (the dim overlay) and `sill idle status` read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Idle {
    pub mode: IdleMode,
    pub phase: IdlePhase,
    /// The dim overlay's wish: every output's dim layer shows `Dimmed` (F902).
    pub dim: DimShown,
    pub power: PowerSource,
    pub inhibition: Inhibition,
    /// `org.freedesktop.ScreenSaver` inhibits held (Q446's `sill idle status`).
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Setup {
    pub display: IdleDisplay,
    /// logind (`Suspend`, `SetIdleHint`, `PrepareForSleep`, `BlockInhibited`).
    pub system: BusRoute,
    /// `org.freedesktop.ScreenSaver`.
    pub session: BusRoute,
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

/// Which bus a half of the service talks to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BusRoute {
    /// The usual system or session bus.
    #[default]
    Usual,
    /// A private bus (`tests/support/private_bus.rs`) with stand-ins.
    Address(String),
    /// None: that half is skipped (no suspend, no `ScreenSaver` name).
    Absent,
}
