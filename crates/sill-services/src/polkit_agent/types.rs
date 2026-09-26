//! The polkit agent's words.

use serde::{Deserialize, Serialize};

use crate::auth::{PamNote, Secret, UserName};

/// polkitd's handle for one authentication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cookie(pub String);

/// The action asked for (`org.freedesktop.systemd1.manage-units`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionId(pub String);

/// A user id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uid(pub u32);

/// A group id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Gid(pub u32);

/// Who may authenticate, as polkitd lists them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Identity {
    UnixUser(Uid),
    UnixGroup(Gid),
    /// Another kind, kept by name and never chosen.
    Other(String),
}

/// One `BeginAuthentication`, parsed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthRequest {
    pub cookie: Cookie,
    pub action_id: ActionId,
    /// The action's own sentence ("Authentication is required to ...").
    pub message: String,
    /// A freedesktop icon name, often empty.
    pub icon: String,
    /// polkitd's details (the program, the unit), in the order sent.
    pub details: Vec<(String, String)>,
    pub identities: Vec<Identity>,
}

/// Whether the helper's prompt shows what is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Echo {
    #[default]
    Hidden,
    Shown,
}

/// Where the shown request is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PromptPhase {
    /// The helper is starting; the field waits.
    #[default]
    Starting,
    /// The helper asked (`Password: `).
    Asking { prompt: String, echo: Echo },
    /// The answer went to the helper.
    Checking,
}

/// How the previous try of this request ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LastTry {
    #[default]
    None,
    /// The helper said `FAILURE`: the surface shakes once.
    Wrong,
}

/// The one request the prompt shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromptView {
    pub cookie: Cookie,
    pub action_id: ActionId,
    pub message: String,
    pub icon: String,
    pub details: Vec<(String, String)>,
    /// Who is asked for (the chosen identity's name).
    pub user: UserName,
    pub phase: PromptPhase,
    pub last: LastTry,
    /// The helper's `PAM_ERROR_MSG` / `PAM_TEXT_INFO` lines for this try.
    pub notes: Vec<PamNote>,
}

/// Whether the agent is registered with polkitd.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Registration {
    /// Not asked to be the agent (`session.polkit_agent` is not `sill`, or no policy yet).
    #[default]
    Passive,
    /// No system bus.
    NoBus,
    Registered,
    /// polkitd refused (another agent holds the session, or no session): its message.
    Refused(String),
}

/// What the polkit prompt follows (`Services.polkit`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Polkit {
    pub registration: Registration,
    /// The request on screen; the others wait in order.
    pub current: Option<PromptView>,
    pub waiting: u32,
}

/// `session.polkit_agent` as the service sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AgentPolicy {
    /// Register and answer (`sill`).
    Own,
    /// Unregister, or never register (anything else).
    #[default]
    Off,
}

/// The secret a [`FakeRequest`]'s stand-in helper accepts (debug builds only).
pub const FAKE_SECRET: &str = "sill-fake";

/// A request made up by `sill debug polkit`: shown like a real one, answered by an in-process
/// stand-in helper that accepts [`FAKE_SECRET`] and nothing else, and completed to nobody.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FakeRequest {
    pub action_id: ActionId,
    pub message: String,
    pub icon: String,
}

/// What the agent service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolkitCommand {
    /// `session.polkit_agent` resolved (at start and on every change).
    Policy(AgentPolicy),
    /// The person's answer to the current request's prompt.
    Answer { cookie: Cookie, secret: Secret },
    /// The person cancelled the request (Cancel, Escape): polkitd hears `Cancelled`.
    Dismiss(Cookie),
    /// Show a made-up request (`sill debug polkit`).
    Fake(FakeRequest),
}
