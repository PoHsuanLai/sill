//! The agent's bus side: connect to polkitd's bus, serve [`Agent`] at [`AGENT_PATH`], find the
//! logind session, and register or unregister with the Authority. Nothing here runs before a
//! `Policy(Own)`.

use std::collections::HashMap;

use tokio::sync::mpsc;
use zbus::Connection;
use zbus::zvariant::{OwnedValue, Value};

use super::dbus::{AGENT_PATH, Agent, AuthorityProxy, Call};
use super::{PolkitBus, SessionChoice, Setup};

/// A polkit subject: `(kind, details)`.
pub(super) type Subject = (String, HashMap<String, OwnedValue>);

/// A connection serving the agent object, and the subject it registers for.
#[derive(Debug)]
pub(super) struct BusLink {
    connection: Connection,
    subject: Subject,
}

/// Why the agent is not registered.
#[derive(Debug)]
pub(super) enum BusTrouble {
    /// No bus to talk to (`Registration::NoBus`).
    NoBus,
    /// polkitd (or logind) said no: its message.
    Refused(String),
}

/// Connect per `setup`, serve the agent object handing calls to `calls`, and find the subject.
pub(super) async fn connect(
    setup: &Setup,
    calls: mpsc::UnboundedSender<Call>,
) -> Result<BusLink, BusTrouble> {
    let connection = open(&setup.bus).await.map_err(|_| BusTrouble::NoBus)?;
    connection
        .object_server()
        .at(AGENT_PATH, Agent { calls })
        .await
        .map_err(|e| BusTrouble::Refused(message_of(&e)))?;
    let id = session_id(&setup.session, &connection)
        .await
        .map_err(|e| BusTrouble::Refused(format!("no logind session: {}", message_of(&e))))?;
    Ok(BusLink {
        connection,
        subject: subject_of(id).map_err(|e| BusTrouble::Refused(e.to_string()))?,
    })
}

async fn open(bus: &PolkitBus) -> zbus::Result<Connection> {
    match bus {
        PolkitBus::System => Connection::system().await,
        PolkitBus::Address(address) => {
            zbus::connection::Builder::address(address.as_str())?
                .build()
                .await
        }
    }
}

async fn session_id(choice: &SessionChoice, connection: &Connection) -> zbus::Result<String> {
    match choice {
        SessionChoice::Id(id) => Ok(id.clone()),
        SessionChoice::Ours => {
            let proxy = zbus::Proxy::new(
                connection,
                "org.freedesktop.login1",
                "/org/freedesktop/login1/session/auto",
                "org.freedesktop.login1.Session",
            )
            .await?;
            proxy.get_property::<String>("Id").await
        }
    }
}

/// `("unix-session", {"session-id": id})`.
fn subject_of(id: String) -> Result<Subject, zbus::zvariant::Error> {
    let id = Value::from(id).try_to_owned()?;
    Ok((
        "unix-session".to_owned(),
        HashMap::from([("session-id".to_owned(), id)]),
    ))
}

/// Register the served object for the link's session.
pub(super) async fn register(link: &BusLink) -> Result<(), BusTrouble> {
    let authority = AuthorityProxy::new(&link.connection)
        .await
        .map_err(|e| BusTrouble::Refused(message_of(&e)))?;
    authority
        .register_authentication_agent(&link.subject, &locale(), AGENT_PATH)
        .await
        .map_err(|e| BusTrouble::Refused(message_of(&e)))
}

/// Unregister; a failure (polkitd gone) leaves nothing to undo.
pub(super) async fn unregister(link: &BusLink) {
    if let Ok(authority) = AuthorityProxy::new(&link.connection).await {
        let _ = authority
            .unregister_authentication_agent(&link.subject, AGENT_PATH)
            .await;
    }
}

/// The locale polkitd words its messages in.
fn locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "C".to_owned())
}

/// A D-Bus error's own message when it has one.
fn message_of(error: &zbus::Error) -> String {
    match error {
        zbus::Error::MethodError(name, Some(text), _) => format!("{name}: {text}"),
        zbus::Error::MethodError(name, None, _) => name.to_string(),
        other => other.to_string(),
    }
}
