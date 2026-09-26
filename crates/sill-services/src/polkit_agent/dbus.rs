//! The agent object sill serves and the Authority it registers with. Private to the service
//! and lane b's to shape: the frozen parts are the D-Bus names and signatures, which are
//! polkit's (`org.freedesktop.PolicyKit1.AuthenticationAgent.xml`), and [`request_from_wire`]'s
//! contract. The skeleton is here so the `#[interface]`, `#[proxy]` and `DBusError` macros are
//! proven to build on the workspace's `zbus` features before anyone starts.

// Freeze: lane b's live loop serves and calls these; until then nothing constructs them.
#![allow(dead_code)]
use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use zbus::zvariant::OwnedValue;
use zbus::{interface, proxy};

use super::machine::Completion;
use super::types::{AuthRequest, Cookie};

/// Where sill serves its agent object (any path is allowed; this one is sill's).
pub const AGENT_PATH: &str = "/org/sill/PolicyKit1/AuthenticationAgent";

/// polkitd's name and its Authority's path.
pub const AUTHORITY_NAME: &str = "org.freedesktop.PolicyKit1";
pub const AUTHORITY_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";

/// A method call the object hands to the live loop.
#[derive(Debug)]
pub enum Call {
    /// `BeginAuthentication`: answered once the request completes.
    Begin {
        request: AuthRequest,
        reply: oneshot::Sender<Completion>,
    },
    /// `CancelAuthentication`.
    Cancel(Cookie),
}

/// polkit's errors, as its agents return them.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.PolicyKit1.Error")]
pub enum AgentError {
    #[zbus(error)]
    ZBus(zbus::Error),
    Failed(String),
    Cancelled(String),
}

/// `BeginAuthentication`'s arguments, parsed: `identities` are `(kind, details)` pairs,
/// `unix-user` with a `uid` (u32) and `unix-group` with a `gid`, anything else kept as
/// `Identity::Other(kind)`, a malformed one dropped; `details` sorted by key.
pub fn request_from_wire(
    action_id: String,
    message: String,
    icon_name: String,
    details: HashMap<String, String>,
    cookie: String,
    identities: Vec<(String, HashMap<String, OwnedValue>)>,
) -> AuthRequest {
    todo!(
        "M11 fill: b-polkit ({action_id} {message} {icon_name} {} {cookie} {})",
        details.len(),
        identities.len()
    )
}

/// The served object.
#[derive(Debug)]
pub struct Agent {
    pub calls: mpsc::UnboundedSender<Call>,
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl Agent {
    /// Waits until the request completes: returns on `Authorized`, fails `Cancelled` or
    /// `Failed` otherwise, as polkitd expects.
    async fn begin_authentication(
        &self,
        action_id: String,
        message: String,
        icon_name: String,
        details: HashMap<String, String>,
        cookie: String,
        identities: Vec<(String, HashMap<String, OwnedValue>)>,
    ) -> Result<(), AgentError> {
        let request = request_from_wire(action_id, message, icon_name, details, cookie, identities);
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(Call::Begin { request, reply })
            .map_err(|_| AgentError::Failed("sill's agent has stopped".to_owned()))?;
        match answer.await {
            Ok(Completion::Authorized) => Ok(()),
            Ok(Completion::Cancelled) => Err(AgentError::Cancelled("cancelled".to_owned())),
            Ok(Completion::Failed) | Err(_) => {
                Err(AgentError::Failed("authentication failed".to_owned()))
            }
        }
    }

    async fn cancel_authentication(&self, cookie: String) -> Result<(), AgentError> {
        self.calls
            .send(Call::Cancel(Cookie(cookie)))
            .map_err(|_| AgentError::Failed("sill's agent has stopped".to_owned()))
    }
}

/// The part of polkitd's Authority an agent calls. A subject is `(kind, details)`: sill sends
/// `("unix-session", {"session-id": <id>})`.
#[proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority",
    gen_blocking = false
)]
pub trait Authority {
    fn register_authentication_agent(
        &self,
        subject: &(String, HashMap<String, OwnedValue>),
        locale: &str,
        object_path: &str,
    ) -> zbus::Result<()>;

    fn unregister_authentication_agent(
        &self,
        subject: &(String, HashMap<String, OwnedValue>),
        object_path: &str,
    ) -> zbus::Result<()>;
}
