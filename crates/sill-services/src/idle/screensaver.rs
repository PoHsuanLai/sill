//! `org.freedesktop.ScreenSaver` on [`Setup::session`](super::Setup), served while idle is
//! `Own`: `Inhibit(app, reason) -> cookie`, `UnInhibit(cookie)` and `GetActive()`, at
//! `/org/freedesktop/ScreenSaver` and `/ScreenSaver`. The object hands each call to the live
//! loop, which owns the inhibitors ([`super::inhibit_step`]); a caller that leaves the bus loses
//! its inhibits (`NameOwnerChanged`). The name is asked for without replacing a holder: if
//! another holds it, its inhibits never reach sill, and that is logged.

use tokio::sync::{mpsc, oneshot, watch};
use zbus::fdo::{DBusProxy, NameOwnerChangedStream, RequestNameFlags, RequestNameReply};
use zbus::message::Header;
use zbus::{Connection, fdo, interface};

use super::inhibit::{BusPeer, InhibitCookie, InhibitMsg, InhibitReply};
use super::logind::bus;
use super::model::{Idle, IdlePhase};
use crate::env::Bus;

/// The well-known name.
pub(super) const BUS_NAME: &str = "org.freedesktop.ScreenSaver";

/// The two paths apps call.
pub(super) const PATHS: [&str; 2] = ["/org/freedesktop/ScreenSaver", "/ScreenSaver"];

/// A call handed to the loop.
#[derive(Debug)]
pub(super) struct Call {
    pub(super) msg: InhibitMsg,
    pub(super) reply: oneshot::Sender<InhibitReply>,
}

/// The served object (one per path).
#[derive(Debug, Clone)]
struct Saver {
    calls: mpsc::UnboundedSender<Call>,
    idle: watch::Receiver<Idle>,
}

fn stopped() -> fdo::Error {
    fdo::Error::Failed("sill's idle service has stopped".to_owned())
}

fn caller(header: &Header<'_>) -> fdo::Result<BusPeer> {
    header
        .sender()
        .map(|name| BusPeer(name.to_string()))
        .ok_or_else(|| fdo::Error::Failed("no sender".to_owned()))
}

impl Saver {
    async fn ask(&self, msg: InhibitMsg) -> fdo::Result<InhibitReply> {
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(Call { msg, reply })
            .map_err(|_| stopped())?;
        answer.await.map_err(|_| stopped())
    }
}

#[interface(name = "org.freedesktop.ScreenSaver")]
impl Saver {
    async fn inhibit(
        &self,
        #[zbus(header)] header: Header<'_>,
        application_name: String,
        reason_for_inhibit: String,
    ) -> fdo::Result<u32> {
        let msg = InhibitMsg::Inhibit {
            peer: caller(&header)?,
            app: application_name,
            reason: reason_for_inhibit,
        };
        match self.ask(msg).await? {
            InhibitReply::Cookie(InhibitCookie(cookie)) => Ok(cookie),
            InhibitReply::Done | InhibitReply::Unknown => Err(stopped()),
        }
    }

    async fn un_inhibit(&self, #[zbus(header)] header: Header<'_>, cookie: u32) -> fdo::Result<()> {
        let msg = InhibitMsg::UnInhibit {
            peer: caller(&header)?,
            cookie: InhibitCookie(cookie),
        };
        // An unknown cookie is ignored, as the spec's servers do.
        self.ask(msg).await.map(|_| ())
    }

    /// Whether the "screen saver" is on: the displays are off for idleness.
    fn get_active(&self) -> bool {
        match self.idle.borrow().phase {
            IdlePhase::DisplaysOff | IdlePhase::Suspending => true,
            IdlePhase::Active | IdlePhase::Dimmed => false,
        }
    }
}

/// The served name and the departures it follows. Dropping it closes the connection, which
/// gives the name back.
pub(super) struct Served {
    _connection: Connection,
    pub(super) departures: Option<NameOwnerChangedStream>,
}

impl std::fmt::Debug for Served {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Served").finish_non_exhaustive()
    }
}

/// Serve the object on `session` and ask for the name.
pub(super) async fn serve(
    session: &Bus,
    calls: mpsc::UnboundedSender<Call>,
    idle: watch::Receiver<Idle>,
) -> Option<Served> {
    let connection = bus(session).await?;
    let saver = Saver { calls, idle };
    for path in PATHS {
        if let Err(error) = connection.object_server().at(path, saver.clone()).await {
            eprintln!("sill: idle: serving {path}: {error}");
            return None;
        }
    }
    match connection
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
        Ok(RequestNameReply::Exists | RequestNameReply::InQueue) | Err(zbus::Error::NameTaken) => {
            eprintln!("sill: idle: {BUS_NAME} is held by another; its inhibits never reach sill")
        }
        Err(error) => eprintln!("sill: idle: requesting {BUS_NAME}: {error}"),
    }
    let departures = match DBusProxy::new(&connection).await {
        Ok(dbus) => dbus.receive_name_owner_changed().await.ok(),
        Err(_) => None,
    };
    Some(Served {
        _connection: connection,
        departures,
    })
}
