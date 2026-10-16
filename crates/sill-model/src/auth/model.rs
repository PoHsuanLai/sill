//! What the lock screen follows: the account and where the check is.

use serde::{Deserialize, Serialize};

use super::{Account, Attempts, AuthFault, PamNote};

/// Where the lock screen's password check is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AuthPhase {
    /// Waiting for a secret.
    #[default]
    Idle,
    /// A check is running.
    Checking,
    /// The last secret was wrong; typing again goes back to `Idle` on the surface's side.
    Wrong,
    /// The last secret was right but the password has expired; it does not unlock.
    Expired,
    /// The secret was right: the daemon unlocks. Stays until `Reset`.
    Accepted,
    /// The last check could not run.
    Unavailable(AuthFault),
}

/// What the lock screen follows (`Services.auth`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Auth {
    pub account: Account,
    pub phase: AuthPhase,
    pub failures: Attempts,
    /// The last check's PAM messages, in order.
    pub notes: Vec<PamNote>,
}
