//! `IdleCommand::Configure` against this session: who owns idle ([`resolve_mode`]). The live loop
//! repeats it every [`RESOLVE_EVERY_S`] seconds.

use sill_model::idle::{IdleConfig, IdlePolicy, resolve_mode};
use sill_model::locker::Host;

/// How often a `Configure`d policy is resolved again.
pub(crate) const RESOLVE_EVERY_S: u64 = 15;

/// `config` in the session whose desktop names are `desktop`.
pub(crate) fn resolve(config: &IdleConfig, desktop: &str) -> IdlePolicy {
    IdlePolicy {
        mode: resolve_mode(Host::of(desktop)),
        times: config.times,
        dim: config.dim,
        lock: config.lock,
        locked_off: config.locked_off,
        grace: config.grace,
    }
}
