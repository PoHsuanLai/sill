//! The utterance's `Ended` signal, unicast to the caller of `Begin` and never replayed: the
//! subscription is made before any `Begin`, for every utterance object, and what arrives before
//! `Begin` has returned (so before the object's path is known) is kept by the loop.

use tokio::sync::mpsc::UnboundedSender;
use voice_wire::{Envelope, UtteranceEnd, VoiceVocab};
use zbus::fdo::DBusProxy;
use zbus::message::Type;
use zbus::{Connection, MatchRule, MessageStream};

use super::run::Inbound;
use crate::stream::next;
use voice_dbus::{UTTERANCE_PREFIX, VOICE_BUS};

/// The body of an `Ended` envelope, `None` when it is another vocabulary or malformed.
fn open(text: &str) -> Option<UtteranceEnd> {
    let envelope: Envelope<UtteranceEnd> = serde_json::from_str(text).ok()?;
    (envelope.vocab == VoiceVocab::CURRENT).then_some(envelope.body)
}

/// The subscription, made: awaiting it is what puts it ahead of the first `Begin`.
pub(super) async fn subscribe(connection: &Connection) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.quire.Voice1.Utterance")?
        .member("Ended")?
        .path_namespace(UTTERANCE_PREFIX)?
        .build();
    MessageStream::for_match_rule(rule, connection, None).await
}

/// Tell `inbound` each `Ended` that voiced itself sent, with the object it names.
pub(super) async fn forward(
    connection: Connection,
    mut stream: MessageStream,
    inbound: UnboundedSender<Inbound>,
) {
    let Ok(bus) = DBusProxy::new(&connection).await else {
        return;
    };
    while let Some(Ok(message)) = next(&mut stream).await {
        let header = message.header();
        let (Some(sender), Some(path)) = (header.sender(), header.path()) else {
            continue;
        };
        let owner = bus
            .get_name_owner(VOICE_BUS.try_into().unwrap_or_else(|_| unreachable!()))
            .await;
        if owner.is_ok_and(|owner| owner.as_str() == sender.as_str())
            && let Ok(text) = message.body().deserialize::<String>()
            && let Some(end) = open(&text)
            && inbound
                .send(Inbound::Signal(path.to_string(), end))
                .is_err()
        {
            return;
        }
    }
}
