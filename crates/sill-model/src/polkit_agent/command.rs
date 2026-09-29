//! What the polkit agent can be told.

use serde::{Deserialize, Serialize};

use super::{ActionId, AgentPolicy, Cookie};
use crate::auth::Secret;

/// A request made up by `sill debug polkit`: shown like a real one, answered by an in-process
/// stand-in helper that accepts `FAKE_SECRET` and nothing else, and completed to nobody.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FakeRequest {
    pub action_id: ActionId,
    pub message: String,
    pub icon: String,
}

/// What the agent service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolkitCommand {
    /// Be the agent or not (the daemon sends `Own` at start).
    Policy(AgentPolicy),
    /// The person's answer to the current request's prompt.
    Answer { cookie: Cookie, secret: Secret },
    /// The person cancelled the request (Cancel, Escape): polkitd hears `Cancelled`.
    Dismiss(Cookie),
    /// Show a made-up request (`sill debug polkit`).
    Fake(FakeRequest),
}
