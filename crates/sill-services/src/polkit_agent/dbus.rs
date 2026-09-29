//! The agent object sill serves and the Authority it registers with. Private to the service: the
//! fixed parts are the D-Bus names and signatures, which are polkit's
//! (`org.freedesktop.PolicyKit1.AuthenticationAgent.xml`), and [`request_from_wire`]'s contract.

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use zbus::zvariant::OwnedValue;
use zbus::{interface, proxy};

use super::machine::Completion;
use super::types::{ActionId, AuthRequest, Cookie, Gid, Identity, Uid};

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
    let mut details: Vec<(String, String)> = details.into_iter().collect();
    details.sort();
    AuthRequest {
        cookie: Cookie(cookie),
        action_id: ActionId(action_id),
        message,
        icon: icon_name,
        details,
        identities: identities.iter().filter_map(identity_of).collect(),
    }
}

/// One `(kind, details)` identity; `None` when a user or group lacks its number.
fn identity_of((kind, details): &(String, HashMap<String, OwnedValue>)) -> Option<Identity> {
    let number = |key: &str| details.get(key).and_then(|v| u32::try_from(v).ok());
    match kind.as_str() {
        "unix-user" => number("uid").map(|uid| Identity::UnixUser(Uid(uid))),
        "unix-group" => number("gid").map(|gid| Identity::UnixGroup(Gid(gid))),
        other => Some(Identity::Other(other.to_owned())),
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use zbus::zvariant::{OwnedValue, Value};

    use super::request_from_wire;
    use crate::polkit_agent::types::{ActionId, Cookie, Gid, Identity, Uid};

    fn owned(value: Value<'_>) -> OwnedValue {
        value
            .try_to_owned()
            .unwrap_or_else(|e| panic!("owned: {e}"))
    }

    #[test]
    fn a_wire_request_parses_its_identities_and_sorts_its_details() {
        let entry = |key: &str, value: Value<'_>| HashMap::from([(key.to_owned(), owned(value))]);
        let identities = vec![
            ("unix-user".to_owned(), entry("uid", Value::U32(1000))),
            ("unix-group".to_owned(), entry("gid", Value::U32(10))),
            ("unix-user".to_owned(), entry("uid", Value::from("x"))),
            ("unix-user".to_owned(), HashMap::new()),
            ("unix-netgroup".to_owned(), HashMap::new()),
        ];
        let details = HashMap::from([
            ("program".to_owned(), "/usr/bin/true".to_owned()),
            ("command_line".to_owned(), "true".to_owned()),
        ]);
        let request = request_from_wire(
            "org.example.run".to_owned(),
            "Run it?".to_owned(),
            String::new(),
            details,
            "c1".to_owned(),
            identities,
        );
        assert_eq!(request.cookie, Cookie("c1".to_owned()));
        assert_eq!(request.action_id, ActionId("org.example.run".to_owned()));
        assert_eq!(
            request.details,
            vec![
                ("command_line".to_owned(), "true".to_owned()),
                ("program".to_owned(), "/usr/bin/true".to_owned()),
            ]
        );
        assert_eq!(
            request.identities,
            vec![
                Identity::UnixUser(Uid(1000)),
                Identity::UnixGroup(Gid(10)),
                Identity::Other("unix-netgroup".to_owned()),
            ]
        );
    }
}
