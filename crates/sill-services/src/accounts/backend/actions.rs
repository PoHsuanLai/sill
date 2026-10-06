//! The notifications server's `ActionInvoked` and `NotificationClosed`, as the answer to the
//! notice's button and the end of the notice.

use sill_model::notifications::{ActionKey, NotificationId};
use zbus::message::Type as MessageType;
use zbus::names::BusName;
use zbus::{Connection, MatchRule, MessageStream};

use crate::notifications::BUS_NAME;

const INTERFACE: &str = "org.freedesktop.Notifications";

/// What the notifications server said about a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Said {
    /// A button was pressed: which notification, and which action of it.
    Pressed { id: NotificationId, key: ActionKey },
    /// The notification closed, for whatever reason.
    Closed(NotificationId),
}

/// Every notifications signal on the bus, whoever sends it, in the order they arrive: a press is
/// heard before the close that follows it.
pub(crate) async fn stream(connection: &Connection) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(INTERFACE)?
        .build();
    MessageStream::for_match_rule(rule, connection, None).await
}

/// What `message` says, when it is the notifications server's own `ActionInvoked` or
/// `NotificationClosed`. A signal from any other connection is nobody's word: the server's name
/// is owned by exactly one connection, and only it says what the person chose. Whether the
/// notification is one sill posted for the account is for the loop to say.
pub(crate) async fn said(connection: &Connection, message: &zbus::Message) -> Option<Said> {
    let header = message.header();
    let said = match header.member()?.as_str() {
        "ActionInvoked" => {
            let (id, key): (u32, String) = message.body().deserialize().ok()?;
            Said::Pressed {
                id: NotificationId(id),
                key: ActionKey(key),
            }
        }
        "NotificationClosed" => {
            let (id, _reason): (u32, u32) = message.body().deserialize().ok()?;
            Said::Closed(NotificationId(id))
        }
        _ => return None,
    };
    let sender = header.sender()?.to_owned();
    let owner = zbus::fdo::DBusProxy::new(connection)
        .await
        .ok()?
        .get_name_owner(BusName::try_from(BUS_NAME).ok()?)
        .await
        .ok()?;
    (owner.as_str() == sender.as_str()).then_some(said)
}
