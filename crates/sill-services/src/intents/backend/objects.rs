//! The objects sill exports for intentd: `org.quire.Confirm1` and one `org.quire.Intents1.Request`
//! per question. They do nothing but check who is calling and hand the call to the service's
//! loop; the loop decides.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use docket_core::{ConfirmId, ConfirmRequest};
use docket_dbus::INTENTS_BUS;
use peephole_bus::peephole_core::roles;
use sill_model::intents::sheet::Serial;
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::fdo;
use zbus::message::Header;
use zbus::names::OwnedUniqueName;
use zbus::zvariant::OwnedObjectPath;

use crate::callers::{Claim, Peers, admit};

/// Where `Confirm` answers a question and where its `Response` goes.
pub(super) const REQUEST_PATH: &str = "/org/quire/Confirm1/request";
/// The interface a request object and its `Response` signal belong to.
pub(super) const REQUEST_INTERFACE: &str = "org.quire.Intents1.Request";

/// Who asked a question: the process that called `Confirm`, and the object its answer goes to.
#[derive(Debug, Clone)]
pub(super) struct Asker {
    pub path: OwnedObjectPath,
    pub to: OwnedUniqueName,
}

/// What reaches the loop from the bus.
#[derive(Debug)]
pub(super) enum Call {
    /// `Confirm`: a question, already given its request object.
    Asked {
        serial: Serial,
        request: Box<ConfirmRequest>,
        asker: Asker,
    },
    /// `Cancel(id)`: the router withdrew a question.
    Cancel(ConfirmId),
    /// `Request.Close()`: the asker stopped waiting.
    Closed(Serial),
}

/// `org.quire.Confirm1`.
pub(super) struct ConfirmObject {
    pub calls: UnboundedSender<Call>,
    pub serials: Arc<AtomicU64>,
    /// Tells the callers on the seat's connection apart.
    pub peers: Arc<Peers>,
}

/// One question's request object. Only its asker may close it.
pub(super) struct RequestObject {
    serial: Serial,
    asker: OwnedUniqueName,
    calls: UnboundedSender<Call>,
}

impl ConfirmObject {
    /// The unique name of the caller, when it is intentd: the program the table names as the
    /// router, which owns intentd's name. Nobody else may ask the person a question in intentd's
    /// name, withdraw one, or answer for it.
    async fn from_intentd(&self, header: &Header<'_>) -> fdo::Result<OwnedUniqueName> {
        let claims = [Claim {
            name: INTENTS_BUS,
            role: roles::ROUTER,
            tag: (),
        }];
        admit(&self.peers, header, &claims)
            .await
            .map(|(name, ())| name)
    }
}

#[zbus::interface(name = "org.quire.Confirm1")]
impl ConfirmObject {
    async fn confirm(
        &self,
        request: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        let asker = self.from_intentd(&header).await?;
        let parsed: ConfirmRequest = serde_json::from_str(&request)
            .map_err(|e| fdo::Error::InvalidArgs(format!("not a ConfirmRequest: {e}")))?;
        let serial = Serial(self.serials.fetch_add(1, Ordering::Relaxed) + 1);
        let path = OwnedObjectPath::try_from(format!("{REQUEST_PATH}/{}", serial.0))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let object = RequestObject {
            serial,
            asker: asker.clone(),
            calls: self.calls.clone(),
        };
        connection.object_server().at(path.as_str(), object).await?;
        let asked = Call::Asked {
            serial,
            request: Box::new(parsed),
            asker: Asker {
                path: path.clone(),
                to: asker,
            },
        };
        self.calls
            .send(asked)
            .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()))?;
        Ok(path)
    }

    async fn cancel(&self, id: String, #[zbus(header)] header: Header<'_>) -> fdo::Result<()> {
        self.from_intentd(&header).await?;
        let id = ConfirmId::parse(&id).map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
        self.calls
            .send(Call::Cancel(id))
            .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()))
    }
}

#[zbus::interface(name = "org.quire.Intents1.Request")]
impl RequestObject {
    async fn close(&self, #[zbus(header)] header: Header<'_>) -> fdo::Result<()> {
        match header.sender() {
            Some(sender) if sender.as_str() == self.asker.as_str() => self
                .calls
                .send(Call::Closed(self.serial))
                .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned())),
            Some(_) | None => Err(fdo::Error::AccessDenied(
                "only the asker closes its request".to_owned(),
            )),
        }
    }
}

/// Take the request object at `path` off the bus.
pub(super) async fn retire(connection: &Connection, path: &OwnedObjectPath) {
    let _ = connection
        .object_server()
        .remove::<RequestObject, _>(path.as_ref())
        .await;
}
