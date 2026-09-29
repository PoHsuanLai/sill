//! The one place sill names shell-host's session lock: `HostCtx::lock_session(LockConfig, factory)
//! -> Result<LockId, LockError::{Unsupported, InProgress}>`, `unlock_session(LockId)`,
//! `lock_surfaces(LockId)`, and `HostEvent::Lock { id, state: Locked | Unlocked | Finished }`.
//! Lock surfaces are made on request, one per output (and for outputs added while locked); a
//! lock document reads `shell_host::use_lock()` (whose `unlock()` sends `unlock_and_destroy` at
//! the next flush) and `use_lock_phase()`. A compositor without `ext-session-lock-v1` answers
//! `Unsupported`.
//!
//! [`SessionLockHost`] is the seam the daemon's lock logic is written against, so
//! `crate::lock`'s table tests drive a fake host and never lock a session.

use std::fmt;

use shell_host::{
    HostCtx, HostEvent, LockConfig, LockError, LockId, LockState, OutputId, OutputInfo, Root,
    SurfaceId,
};

use crate::msg::ShellMsg;

/// The lock surfaces' name prefix: each is `sill-lock/<output>` (shell-host's naming), which
/// `sill debug capture` and `debug stats` address.
pub const LOCK_NAMESPACE: &str = "sill-lock";

/// Builds each output's lock surface root (the daemon's `surface_root` around
/// `sill_surfaces::LockScreen`).
pub struct LockFactory(pub Box<dyn Fn(OutputInfo) -> Root>);

impl fmt::Debug for LockFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LockFactory(..)")
    }
}

/// Why a lock was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockRefusal {
    /// The compositor offers no `ext-session-lock-v1`.
    Unsupported,
    /// A lock of ours is still in force.
    InProgress(LockId),
}

/// The session lock, as the daemon uses it.
pub trait SessionLockHost {
    /// Lock the session with one surface per output from `factory`.
    fn lock_session(&mut self, factory: LockFactory) -> Result<LockId, LockRefusal>;
    /// Unlock `id` (its documents are told `Unlocking`, then `unlock_and_destroy`).
    fn unlock_session(&mut self, id: LockId);
    /// The output lock `id`'s surface `surface` is on, for moving the prompt to the surface
    /// with the keyboard.
    fn lock_surface_output(&self, id: LockId, surface: SurfaceId) -> Option<OutputId>;
}

impl SessionLockHost for HostCtx<ShellMsg> {
    fn lock_session(&mut self, factory: LockFactory) -> Result<LockId, LockRefusal> {
        let config = LockConfig {
            namespace: LOCK_NAMESPACE.to_owned(),
        };
        let LockFactory(build) = factory;
        HostCtx::lock_session(self, config, build).map_err(refusal_of)
    }

    fn unlock_session(&mut self, id: LockId) {
        HostCtx::unlock_session(self, id);
    }

    fn lock_surface_output(&self, id: LockId, surface: SurfaceId) -> Option<OutputId> {
        self.lock_surfaces(id)
            .into_iter()
            .find(|(_, s)| *s == surface)
            .map(|(output, _)| output)
    }
}

/// shell-host's error as sill's refusal.
pub fn refusal_of(error: LockError) -> LockRefusal {
    match error {
        LockError::Unsupported => LockRefusal::Unsupported,
        LockError::InProgress(id) => LockRefusal::InProgress(id),
    }
}

/// `event` as a lock event, when it is one.
pub fn lock_event(event: &HostEvent<ShellMsg>) -> Option<(LockId, LockState)> {
    match event {
        HostEvent::Lock { id, state } => Some((*id, *state)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use shell_host::{HostEvent, LockError, LockId, LockState, OutputId};

    use super::{LockRefusal, lock_event, refusal_of};
    use crate::msg::ShellMsg;

    #[test]
    fn shell_hosts_words_map_onto_sills() {
        assert_eq!(refusal_of(LockError::Unsupported), LockRefusal::Unsupported);
        assert_eq!(
            refusal_of(LockError::InProgress(LockId(3))),
            LockRefusal::InProgress(LockId(3))
        );
        let locked: HostEvent<ShellMsg> = HostEvent::Lock {
            id: LockId(3),
            state: LockState::Locked,
        };
        assert_eq!(lock_event(&locked), Some((LockId(3), LockState::Locked)));
        let other: HostEvent<ShellMsg> = HostEvent::OutputRemoved(OutputId(1));
        assert_eq!(lock_event(&other), None);
    }
}
