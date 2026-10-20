//! The bus half of the live loop: the connection, who holds `org.freedesktop.Notifications`
//! (`GetNameOwner`, `GetConnectionUnixProcessID`, `/proc/<pid>/comm`), and asking for or giving
//! back the name (`DoNotQueue`, never `ReplaceExisting`, and without `AllowReplacement`: another
//! program cannot take the name from sill and read what the apps send).
//!
//! The holder is always read back from the bus rather than inferred from a reply, so a
//! `NameOwnerChanged`, a `NameLost` (another daemon replaced sill) and a policy change all
//! settle the same way: [`Bus::settle`].

use sill_model::notifications::{
    ActionKey, Activation, CloseReason, NameHolder, NameOwner, NotificationId, ServerMode,
};
use sill_model::serve::ClaimError;
use tokio::sync::mpsc;
use zbus::fdo::{DBusProxy, NameOwnerChangedStream};
use zbus::names::BusName;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, fdo};

use super::dbus::{BUS_NAME, Call, OBJECT_PATH, Sender, Server};
use super::guard::Guarded;
use crate::serve;

/// A connection to the bus the service speaks on.
#[derive(Debug)]
pub(crate) struct Bus {
    connection: Connection,
    dbus: DBusProxy<'static>,
    /// Where the served object sends its calls.
    calls: mpsc::UnboundedSender<Call>,
}

impl Bus {
    /// Connect to `bus`, with the `NameOwnerChanged` stream for the name; `None` when the bus
    /// is absent or unreachable (the service then runs without one: `sill debug notify` works).
    pub(crate) async fn connect(
        bus: &crate::env::Bus,
        calls: mpsc::UnboundedSender<Call>,
    ) -> Option<(Bus, Option<NameOwnerChangedStream>)> {
        let connection = match bus.try_connect().await {
            Ok(connection) => connection,
            Err(crate::env::ConnectError::NoBus(_)) => return None,
            Err(error) => {
                eprintln!("sill: notifications: no bus: {error}");
                return None;
            }
        };
        let dbus = DBusProxy::new(&connection).await.ok()?;
        let changes = dbus
            .receive_name_owner_changed_with_args(&[(0, BUS_NAME)])
            .await
            .ok();
        Some((
            Bus {
                connection,
                dbus,
                calls,
            },
            changes,
        ))
    }

    /// Ask for or give back the name as `mode` says, then report who holds it.
    pub(crate) async fn settle(&self, mode: ServerMode) -> NameHolder {
        let holder = self.holder().await;
        match (mode, &holder) {
            (ServerMode::Own, NameHolder::Nobody) => {
                self.claim().await;
                self.holder().await
            }
            (ServerMode::Passive, NameHolder::Sill) => {
                self.release().await;
                self.holder().await
            }
            _ => holder,
        }
    }

    /// Who holds the name now.
    async fn holder(&self) -> NameHolder {
        let Ok(name) = BusName::try_from(BUS_NAME) else {
            return NameHolder::NoBus;
        };
        let unique = match self.dbus.get_name_owner(name).await {
            Ok(unique) => unique,
            Err(fdo::Error::NameHasNoOwner(_)) => return NameHolder::Nobody,
            Err(_) => return NameHolder::NoBus,
        };
        if self.connection.unique_name().map(|ours| ours.as_str()) == Some(unique.as_str()) {
            return NameHolder::Sill;
        }
        let pid = match BusName::try_from(unique.as_str()) {
            Ok(name) => self.dbus.get_connection_unix_process_id(name).await.ok(),
            Err(_) => None,
        };
        NameHolder::Other(NameOwner {
            unique: unique.to_string(),
            pid,
            command: pid.and_then(command_of),
        })
    }

    /// Serve the object and ask for the name, never replacing a holder.
    async fn claim(&self) {
        let server = Guarded::new(Server {
            calls: self.calls.clone(),
        });
        if let Err(error) = self
            .connection
            .object_server()
            .at(OBJECT_PATH, server)
            .await
        {
            eprintln!("sill: notifications: serving {OBJECT_PATH}: {error}");
            return;
        }
        match serve::take_name(&self.connection, BUS_NAME).await {
            // Taken is not an error: the holder is read back and shown.
            Ok(()) | Err(ClaimError::NameTaken(_)) => {}
            Err(error) => eprintln!("sill: notifications: requesting {BUS_NAME}: {error:?}"),
        }
    }

    /// Give the name back and stop serving the object.
    pub(crate) async fn release(&self) {
        if let Err(error) = self.connection.release_name(BUS_NAME).await {
            eprintln!("sill: notifications: releasing {BUS_NAME}: {error}");
        }
        let _ = self
            .connection
            .object_server()
            .remove::<Guarded, _>(OBJECT_PATH)
            .await;
    }

    /// Emit `NotificationClosed`.
    pub(crate) async fn closed(&self, id: NotificationId, reason: CloseReason) {
        let Ok(emitter) = SignalEmitter::new(&self.connection, OBJECT_PATH) else {
            return;
        };
        if let Err(error) = Server::notification_closed(&emitter, id.0, reason.wire()).await {
            eprintln!("sill: notifications: NotificationClosed: {error}");
        }
    }

    /// Emit `ActivationToken` (when there is one), then `ActionInvoked`, to `to`, the connection
    /// that posted the notification. The token lets its holder raise a window, so it goes to no
    /// one else: a notification that has no connection (the shell's own) gets none, and its
    /// `ActionInvoked` is the broadcast the shell's own services listen for.
    pub(crate) async fn invoked(
        &self,
        id: NotificationId,
        action: &ActionKey,
        token: &Activation,
        to: Option<&Sender>,
    ) {
        let Ok(emitter) = SignalEmitter::new(&self.connection, OBJECT_PATH) else {
            return;
        };
        let emitter = match to.map(|to| BusName::try_from(to.0.clone())) {
            Some(Ok(name)) => emitter.set_destination(name),
            Some(Err(_)) => return,
            None => emitter,
        };
        if let (Activation::Token(token), Some(_)) = (token, to)
            && let Err(error) = Server::activation_token(&emitter, id.0, &token.0).await
        {
            eprintln!("sill: notifications: ActivationToken: {error}");
        }
        if let Err(error) = Server::action_invoked(&emitter, id.0, &action.0).await {
            eprintln!("sill: notifications: ActionInvoked: {error}");
        }
    }
}

/// `/proc/<pid>/comm`, trimmed.
fn command_of(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|comm| comm.trim().to_owned())
        .filter(|comm| !comm.is_empty())
}
