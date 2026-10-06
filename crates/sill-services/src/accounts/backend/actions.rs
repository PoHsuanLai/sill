//! The notifications server's `ActionInvoked`, as the answer to the notice's button.

use sill_model::accounts::{AccountKey, account_of};
use sill_model::notifications::ActionKey;
use zbus::message::Type as MessageType;
use zbus::names::BusName;
use zbus::{Connection, MatchRule, MessageStream};

use crate::notifications::BUS_NAME;

const INTERFACE: &str = "org.freedesktop.Notifications";

/// Every `ActionInvoked` on the bus, whoever sends it.
pub(crate) async fn stream(connection: &Connection) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(INTERFACE)?
        .member("ActionInvoked")?
        .build();
    MessageStream::for_match_rule(rule, connection, None).await
}

/// The account whose button was pressed, when `message` is the notifications server's own
/// `ActionInvoked` for a sign-in button. A signal from any other connection, or for any other
/// key, is nobody's answer: the server's name is owned by exactly one connection, and only it
/// says what the person chose.
pub(crate) async fn pressed(
    connection: &Connection,
    message: &zbus::Message,
) -> Option<AccountKey> {
    let (_id, key): (u32, String) = message.body().deserialize().ok()?;
    let account = account_of(&ActionKey(key))?;
    let sender = message.header().sender()?.to_owned();
    let owner = zbus::fdo::DBusProxy::new(connection)
        .await
        .ok()?
        .get_name_owner(BusName::try_from(BUS_NAME).ok()?)
        .await
        .ok()?;
    (owner.as_str() == sender.as_str()).then_some(account)
}
