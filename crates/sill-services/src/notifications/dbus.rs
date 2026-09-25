//! The `org.freedesktop.Notifications` object, served at `/org/freedesktop/Notifications` once
//! the service holds the name, and the parse from the wire's arguments into a [`Posting`].
//!
//! Private to the service and lane a's to shape (FINDINGS "The M6 fill-in"): the frozen parts
//! are the D-Bus names and signatures below, which are the spec's, and [`posting_from_wire`]'s
//! contract. The skeleton is here so the `#[interface]` macro and signal emitters are proven to
//! build on the workspace's `zbus` features before any agent starts (dependencies first).

// The live loop (`live.rs`, lane a) constructs and serves `Server`; until it does, nothing here
// is reached. Remove this line when it is.
#![allow(dead_code)]

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use super::capabilities::{CAPABILITIES, SERVER_INFORMATION};
use super::content::Posting;
use super::ids::NotificationId;

/// The well-known name.
pub const BUS_NAME: &str = "org.freedesktop.Notifications";

/// The object path the spec fixes.
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

/// A method call the object hands to the live loop, which owns the state.
#[derive(Debug)]
pub enum Call {
    /// `Notify`: the id is sent back on `reply` once the loop has assigned it.
    Notify {
        posting: Box<Posting>,
        reply: oneshot::Sender<NotificationId>,
    },
    /// `CloseNotification`.
    Close(NotificationId),
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

/// `call` parsed: the body's markup into runs ([`super::markup::parse_body`]); the actions
/// paired; `urgency` (byte), `category`, `desktop-entry` (a trailing `.desktop` dropped),
/// `image-data` / `image_data` / `icon_data` (converted to straight RGBA; a malformed one is
/// no image), else `image-path`, `transient` and `resident` (booleans; some senders send a
/// byte or an int, read as non-zero), `sound-name`, `sound-file`, `suppress-sound`. Unknown
/// hints are ignored; a hint of the wrong type is as if absent.
pub fn posting_from_wire(call: WireNotify) -> Posting {
    let _ = call;
    todo!("M6 fill: a-server")
}

/// The served object.
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
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let _ = WireNotify {
            app_name,
            replaces_id,
            app_icon,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
        };
        todo!("M6 fill: a-server")
    }

    /// `CloseNotification(u)`.
    async fn close_notification(&self, id: u32) {
        let _ = id;
        todo!("M6 fill: a-server")
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
    use crate::notifications::content::{
        IconRef, NoteImage, NotifyUrgency, Pixels, Residency, SoundHint, Transience,
    };
    use crate::notifications::ids::NotificationId;

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

    #[test]
    #[ignore = "M6 fill: a-server"]
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
        let posting = posting_from_wire(call(vec![
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
                rgba: vec![10, 20, 30, 255],
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
        let posting = posting_from_wire(replacing);
        assert_eq!(posting.replaces, Some(NotificationId(5)));
        assert_eq!(
            posting.hints.image,
            NoteImage::Icon(IconRef::File(PathBuf::from("/tmp/face.png")))
        );
        assert_eq!(posting.hints.sound, SoundHint::Suppressed);
        assert_eq!(posting.hints.urgency, NotifyUrgency::Normal, "wrong type");
    }
}
