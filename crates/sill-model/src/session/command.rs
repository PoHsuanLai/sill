//! What the session service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{Session, SessionAction};

/// One thing the service learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SessionEvent {
    /// A fresh probe: every offer at once.
    Probed(Session),
    /// logind is unreachable.
    Gone,
}

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
