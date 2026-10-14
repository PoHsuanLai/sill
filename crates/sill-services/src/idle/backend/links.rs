//! What the idle service reads and drives, and where it connects.

use sill_model::auth::AuthCommand;
use sill_model::battery::Battery;
use sill_model::locker::{Lock, LockerCommand};
use tokio::sync::watch;

use crate::control::Control;
use crate::env::{Bus, Env, WaylandRoute};

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
#[derive(Debug, Clone)]
pub(crate) struct Reach {
    /// The compositor to hold idle notifications and output power on.
    pub(crate) display: WaylandRoute,
    /// logind (`Suspend`, `SetIdleHint`, `PrepareForSleep`, `BlockInhibited`).
    pub(crate) system: Bus,
    /// `org.freedesktop.ScreenSaver`.
    pub(crate) session: Bus,
    /// The session's desktop names (`XDG_CURRENT_DESKTOP`), which say whose session this is.
    pub(crate) current_desktop: String,
}

impl Reach {
    /// `env`'s compositor, buses and session.
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            display: env.wayland.clone(),
            system: env.buses.system.clone(),
            session: env.buses.session.clone(),
            current_desktop: env.current_desktop.clone(),
        }
    }
}
