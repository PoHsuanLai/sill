//! What the idle service hears and what it can be told.

use super::policy::{IdleConfig, IdlePolicy};

/// What the idle service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleCommand {
    /// The settings (at start and on every change), resolved by the service against this
    /// session (sill's own, or a guest). The service is passive until the first one.
    Configure(IdleConfig),
    /// An already resolved policy (live tests on the nested compositor).
    Use(IdlePolicy),
    /// A made-up `resumed`, then every notification held afresh (`sill debug idle-wake`, sent
    /// only by a daemon built with `debug`): the nested compositor counts no synthetic input
    /// as activity.
    FakeWake,
}
