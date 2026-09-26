//! The one place sill names shell-host's session lock, so only this file changes when it lands
//! (FINDINGS "M11 freeze" F531).
//!
//! Announced on shell-host's `session-lock` branch (not on its master at the freeze):
//! `HostCtx::lock_session(LockConfig, factory per output) -> Result<LockId, LockError>`,
//! `HostCtx::unlock_session(LockId)`, `HostEvent::Lock { id, state: Locked | Finished }`, a
//! surface-side `use_lock()` with `unlock()`, and `LockError::Unsupported` when the compositor
//! has no `ext-session-lock-v1`. sill unlocks from the daemon (after the auth service's
//! `Accepted`), never from a surface, so `use_lock()` is not needed. Until the branch lands
//! [`SessionLockHost`] for `HostCtx` refuses with `Unsupported` and [`lock_event`] sees nothing,
//! so `session.locker = "sill"` locks nothing and says so (`sill lock` then leaves the service
//! `Unlocked`), and the session is never locked by a half-built lock screen.

use std::fmt;

use shell_host::{HostCtx, HostEvent, OutputId, OutputInfo, Root, SurfaceId};

use crate::msg::ShellMsg;

/// Builds each output's lock surface root (the daemon's `surface_root` around
/// `sill_surfaces::LockScreen`).
pub struct LockFactory(pub Box<dyn Fn(&OutputInfo) -> Root>);

impl fmt::Debug for LockFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LockFactory(..)")
    }
}

/// sill's handle for one held lock (shell-host's `LockId` once it lands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockTicket(pub u64);

/// Why a lock was not taken.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockRefusal {
    /// No `ext-session-lock-v1` (or shell-host without its session lock yet).
    Unsupported,
    /// Anything else, as shell-host words it.
    Failed(String),
}

/// What the compositor said about a held lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostLockState {
    /// `ext_session_lock_v1.locked`: every output is covered.
    Locked,
    /// `finished`, or the unlock completed: the lock surfaces are gone.
    Finished,
}

/// The session lock, as the daemon uses it.
pub trait SessionLockHost {
    /// Lock the session with one surface per output from `factory`.
    fn lock_session(&mut self, factory: LockFactory) -> Result<LockTicket, LockRefusal>;
    /// Unlock and destroy the lock surfaces (`unlock_and_destroy`).
    fn unlock_session(&mut self, ticket: LockTicket);
    /// The output a lock surface is on, for moving the prompt to the surface with the keyboard.
    fn lock_surface_output(&self, id: SurfaceId) -> Option<OutputId>;
}

impl SessionLockHost for HostCtx<ShellMsg> {
    fn lock_session(&mut self, factory: LockFactory) -> Result<LockTicket, LockRefusal> {
        let _ = factory;
        Err(LockRefusal::Unsupported)
    }

    fn unlock_session(&mut self, ticket: LockTicket) {
        let _ = ticket;
    }

    fn lock_surface_output(&self, id: SurfaceId) -> Option<OutputId> {
        let _ = id;
        None
    }
}

/// `event` as a lock event, when it is one (`HostEvent::Lock` once it lands; nothing until then).
pub fn lock_event(event: &HostEvent<ShellMsg>) -> Option<(LockTicket, HostLockState)> {
    let _ = event;
    None
}
