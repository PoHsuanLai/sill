//! The session: logind's power actions and what it currently allows (design/20 §1.8: the power
//! menu's Log Out, Restart, Shut Down and Suspend; the launcher's system rows carry out through
//! the same control).
//!
//! logind is spoken through `logind-zbus` (already a dependency, the launcher's system provider
//! uses it for `Can*`). The provider keeps its own availability snapshot for now (its
//! `OnceLock`, taken on the first query): moving it onto this service's receiver is a change to
//! the provider's constructor, which the results runner and the launcher panel both call, and
//! nothing is gained until the power menu needs the two to agree live.

mod live;
mod model;

pub use model::{Offer, Session, SessionAction, SessionEvent, reduce_session};

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::{Bus, SessionActions};
use crate::locker::LockerCommand;

/// What the session service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    /// Carry out `action`: `Lock` goes to the locker service (which runs the configured
    /// locker, or asks logind's `LockSession` for COSMIC's resident one), `LogOut`
    /// `TerminateSession`, the rest the manager's `Suspend`/`Reboot`/`PowerOff` with
    /// `interactive = true` (polkit may ask). An action logind does not offer is dropped.
    Run(SessionAction),
    /// Re-read every `Can*` (after a resume, or before the power menu opens).
    Refresh,
}

/// Start the session task on `rt`. Without logind the receiver stays at `Session::default()`
/// (absent: nothing offered), and the power menu shows every button unavailable. `locker` is
/// where `Run(Lock)` goes.
pub fn spawn(
    rt: &Handle,
    bus: Bus,
    actions: Arc<dyn SessionActions>,
    locker: Control<LockerCommand>,
) -> (watch::Receiver<Session>, Control<SessionCommand>) {
    let (tx, rx) = watch::channel(Session::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(bus, actions, tx, commands, locker));
    (rx, control)
}
