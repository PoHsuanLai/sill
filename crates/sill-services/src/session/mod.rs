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

pub use model::{Offer, Session, SessionAction, SessionEvent, step};

use std::sync::Arc;

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::locker::LockerCommand;
use crate::service::{Service, ServiceIo};

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

/// logind's power actions as a [`Service`]. Without logind the state stays at
/// `Session::default()` (absent: nothing offered), and the power menu shows every button
/// unavailable. Its `Deps` is the locker's control, where `Run(Lock)` goes; the other actions go
/// to `env`'s session actions.
#[derive(Debug)]
pub enum SessionService {}

impl Service for SessionService {
    const NAME: &'static str = "session";
    type State = Session;
    type Command = SessionCommand;
    type Deps = Control<LockerCommand>;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.system.clone();
        let actions = Arc::clone(&env.session_actions);
        async move {
            live::run(bus, actions, io.state, io.commands, deps).await;
            Ok(())
        }
    }
}
