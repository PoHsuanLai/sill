//! The `org.freedesktop.Notifications` object, served at `/org/freedesktop/Notifications` once
//! the service holds the name, and the parse from the wire's arguments into a [`Posting`].
//!
//! Private to the service: the fixed parts are the D-Bus names and signatures below, which are the
//! spec's, and [`posting_from_wire`]'s contract.

use std::collections::HashMap;
use std::time::Duration;

use sill_model::notifications::{CAPABILITIES, NotificationId, Posting, SERVER_INFORMATION};
use tokio::sync::{mpsc, oneshot};
use zbus::fdo::DBusProxy;
use zbus::message::Header;
use zbus::names::BusName;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, fdo, interface};

use crate::deadline::within;

/// The well-known name.
pub const BUS_NAME: &str = "org.freedesktop.Notifications";

/// The object path the spec fixes.
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

/// A caller's unique connection name (`:1.42`): who a notification belongs to. Only its sender
/// may replace or close it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sender(pub String);

/// Who a `Notify` is counted against: the program behind the connection, so that a program that
/// closes its connection and opens another is the same caller and gets no fresh allowance. A
/// connection whose program the bus cannot name is counted by itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Caller {
    /// The process id the bus reports for the connection.
    Process(u32),
    /// The connection alone.
    Connection(Sender),
}

/// How long the bus is given to say which process a connection belongs to: it answers from its own
/// table, so a slow answer means a stuck bus and the connection is counted by itself.
const CREDENTIALS_CALL: Duration = Duration::from_secs(2);

/// Who `sender` is, for the allowance: its process when the bus says in time.
async fn caller_of(connection: &Connection, sender: &Sender) -> Caller {
    let alone = || Caller::Connection(sender.clone());
    let Ok(name) = BusName::try_from(sender.0.as_str()) else {
        return alone();
    };
    let Ok(dbus) = DBusProxy::new(connection).await else {
        return alone();
    };
    match within(
        CREDENTIALS_CALL,
        "notification caller",
        dbus.get_connection_unix_process_id(name),
    )
    .await
    {
        Ok(Ok(pid)) => Caller::Process(pid),
        Ok(Err(_)) | Err(_) => alone(),
    }
}

/// A method call the object hands to the live loop, which owns the state.
#[derive(Debug)]
pub enum Call {
    /// `Notify`: the id is sent back on `reply` once the loop has assigned it.
    Notify {
        posting: Box<Posting>,
        sender: Sender,
        caller: Caller,
        reply: oneshot::Sender<NotifyAnswer>,
    },
    /// `CloseNotification`: the loop answers whether it held the id for `sender`.
    Close {
        id: NotificationId,
        sender: Sender,
        reply: oneshot::Sender<CloseAnswer>,
    },
}

/// What `Notify` finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyAnswer {
    /// Held under this id.
    Posted(NotificationId),
    /// The sender has posted too many too fast: refused.
    TooMany,
}

/// What `CloseNotification` finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAnswer {
    /// Closed, and `NotificationClosed` (reason 3) emitted.
    Closed,
    /// Not held, or held for another sender: the spec's empty D-Bus error.
    NotHeld,
}

/// Why a `Notify` was turned away before it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireRefusal {
    /// More than `HINTS_MAX` hints.
    TooManyHints,
}

/// `Notify`'s eight arguments as they arrived.
#[derive(Debug, Clone, PartialEq)]
pub struct WireNotify {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: HashMap<String, OwnedValue>,
    pub expire_timeout: i32,
}

/// `call` parsed: the body's markup into runs ([`sill_model::notifications::parse_body`]); the actions
/// paired; `urgency` (byte), `category`, `desktop-entry` (a trailing `.desktop` dropped),
/// `image-data` / `image_data` / `icon_data` (converted to straight RGBA; a malformed one is
/// no image), else `image-path`, `transient` and `resident` (booleans; some senders send a
/// byte or an int, read as non-zero), `sound-name`, `sound-file`, `suppress-sound`. Unknown
/// hints are ignored; a hint of the wrong type is as if absent. Every field is cut to its ceiling
/// (`sill_model::notifications::limits`); more than `HINTS_MAX` hints is refused.
pub fn posting_from_wire(call: WireNotify) -> Result<Posting, WireRefusal> {
    super::wire::posting(call)
}

fn stopped() -> fdo::Error {
    fdo::Error::Failed("the notifications service has stopped".to_owned())
}

/// The unique name of whoever made the call.
fn sender_of(header: &Header<'_>) -> fdo::Result<Sender> {
    header
        .sender()
        .map(|name| Sender(name.to_string()))
        .ok_or_else(|| fdo::Error::Failed("no sender".to_owned()))
}

/// The served object's methods and signals. It is served behind [`super::guard::Guarded`], which
/// refuses an oversize `Notify` before this reads its arguments.
#[derive(Debug)]
pub struct Server {
    pub calls: mpsc::UnboundedSender<Call>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl Server {
    /// `Notify(susssasa{sv}i) -> u`.
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> fdo::Result<u32> {
        let sender = sender_of(&header)?;
        let caller = caller_of(connection, &sender).await;
        let posting = posting_from_wire(WireNotify {
            app_name,
            replaces_id,
            app_icon,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
        })
        .map_err(|WireRefusal::TooManyHints| {
            fdo::Error::InvalidArgs("too many hints".to_owned())
        })?;
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(Call::Notify {
                posting: Box::new(posting),
                sender,
                caller,
                reply,
            })
            .map_err(|_| stopped())?;
        match answer.await.map_err(|_| stopped())? {
            NotifyAnswer::Posted(id) => Ok(id.0),
            NotifyAnswer::TooMany => Err(fdo::Error::LimitsExceeded(
                "too many notifications too fast".to_owned(),
            )),
        }
    }

    /// `CloseNotification(u)`: an id no longer held, or held for another sender, answers the
    /// spec's empty error.
    async fn close_notification(
        &self,
        #[zbus(header)] header: Header<'_>,
        id: u32,
    ) -> fdo::Result<()> {
        let sender = sender_of(&header)?;
        let (reply, answer) = oneshot::channel();
        self.calls
            .send(Call::Close {
                id: NotificationId(id),
                sender,
                reply,
            })
            .map_err(|_| stopped())?;
        match answer.await.map_err(|_| stopped())? {
            CloseAnswer::Closed => Ok(()),
            CloseAnswer::NotHeld => Err(fdo::Error::Failed(String::new())),
        }
    }

    /// `GetCapabilities() -> as`.
    fn get_capabilities(&self) -> Vec<String> {
        CAPABILITIES.iter().map(|word| (*word).to_owned()).collect()
    }

    /// `GetServerInformation() -> ssss`.
    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    fn get_server_information(&self) -> (String, String, String, String) {
        let info = SERVER_INFORMATION;
        (
            info.name.to_owned(),
            info.vendor.to_owned(),
            info.version.to_owned(),
            info.spec_version.to_owned(),
        )
    }

    /// `NotificationClosed(uu)`.
    #[zbus(signal)]
    pub async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// `ActionInvoked(us)`.
    #[zbus(signal)]
    pub async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;

    /// `ActivationToken(us)` (spec 1.2), emitted before `ActionInvoked`.
    #[zbus(signal)]
    pub async fn activation_token(
        emitter: &SignalEmitter<'_>,
        id: u32,
        activation_token: &str,
    ) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use zbus::zvariant::{OwnedValue, Value};

    use super::{WireNotify, posting_from_wire};
    use sill_model::notifications::NotificationId;
    use sill_model::notifications::{
        IconRef, NoteImage, NotifyUrgency, Pixels, Posting, Residency, SoundHint, Transience,
    };

    fn owned(value: Value<'_>) -> OwnedValue {
        value.try_into_owned().unwrap_or_else(|e| panic!("{e}"))
    }

    fn call(hints: Vec<(&str, OwnedValue)>) -> WireNotify {
        WireNotify {
            app_name: "Mail".to_owned(),
            replaces_id: 0,
            app_icon: "mail-unread".to_owned(),
            summary: "Ada".to_owned(),
            body: "the <b>engine</b>".to_owned(),
            actions: ["default", "Open", "reply", "Reply"]
                .map(str::to_owned)
                .to_vec(),
            hints: hints
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<HashMap<_, _>>(),
            expire_timeout: -1,
        }
    }

    fn posting_of(call: WireNotify) -> Posting {
        posting_from_wire(call).unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn the_wire_becomes_a_posting() {
        let rgb = owned(Value::from((
            1i32,
            1i32,
            3i32,
            false,
            8i32,
            3i32,
            vec![10u8, 20, 30],
        )));
        let posting = posting_of(call(vec![
            ("urgency", owned(Value::from(2u8))),
            ("category", owned(Value::from("email.arrived"))),
            (
                "desktop-entry",
                owned(Value::from("org.example.Mail.desktop")),
            ),
            ("image-data", rgb),
            ("transient", owned(Value::from(true))),
            ("resident", owned(Value::from(1u8))),
            ("sound-name", owned(Value::from("message-new-email"))),
            ("x-unknown", owned(Value::from(7u32))),
        ]));
        assert_eq!(posting.replaces, None);
        assert_eq!(posting.app.name, "Mail");
        assert_eq!(posting.app.icon, IconRef::Name("mail-unread".to_owned()));
        assert_eq!(
            posting.app.desktop_entry.as_deref(),
            Some("org.example.Mail")
        );
        assert_eq!(posting.body.text(), "the engine");
        assert_eq!(posting.actions.len(), 2);
        assert_eq!(posting.hints.urgency, NotifyUrgency::Critical);
        assert_eq!(posting.hints.category.as_deref(), Some("email.arrived"));
        assert_eq!(
            posting.hints.image,
            NoteImage::Pixels(Pixels {
                width: 1,
                height: 1,
                rgba: vec![10, 20, 30, 255].into(),
            })
        );
        assert_eq!(posting.hints.transience, Transience::Transient);
        assert_eq!(posting.hints.residency, Residency::Resident);
        assert_eq!(
            posting.hints.sound,
            SoundHint::Named("message-new-email".to_owned())
        );

        let mut replacing = call(vec![
            ("image-path", owned(Value::from("file:///tmp/face.png"))),
            ("suppress-sound", owned(Value::from(true))),
            ("urgency", owned(Value::from("high"))),
        ]);
        replacing.replaces_id = 5;
        let posting = posting_of(replacing);
        assert_eq!(posting.replaces, Some(NotificationId(5)));
        assert_eq!(
            posting.hints.image,
            NoteImage::Icon(IconRef::File(PathBuf::from("/tmp/face.png")))
        );
        assert_eq!(posting.hints.sound, SoundHint::Suppressed);
        assert_eq!(posting.hints.urgency, NotifyUrgency::Normal, "wrong type");
    }
}
