//! The utterance's `Ended` signal, unicast to the caller of `Begin` and never replayed: the
//! subscription is made before any `Begin`, for every utterance object, and what arrives before
//! `Begin` has returned (so before the object's path is known) is kept by the loop.

use std::sync::Arc;

use peephole_bus::peephole_core::RoleName;
use tokio::sync::mpsc::UnboundedSender;
use voice_wire::{Envelope, UtteranceEnd, VoiceVocab};
use zbus::message::Type;
use zbus::{Connection, MatchRule, MessageStream};

use super::run::Inbound;
use crate::callers::{Claim, Peers, SignalSenders};
use crate::stream::next;
use voice_dbus::{UTTERANCE_PREFIX, VOICE_BUS};

/// The role the caller table gives voiced's program. Owning `org.quire.Voice1` proves nothing: any
/// process may claim a name nobody owns yet, so a signal or a call counts as voiced's only when
/// the system says the process behind the name is voiced.
const VOICED_ROLE: RoleName = RoleName::from_static("voice");

/// voiced, by the name it owns and the role it plays.
pub(super) fn voiced() -> Claim<()> {
    Claim {
        name: VOICE_BUS,
        role: VOICED_ROLE,
        tag: (),
    }
}

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

/// Tell `inbound` each `Ended` that voiced itself sent, with the object it names. A signal from
/// any other program, whatever name it holds, is dropped, and said once.
pub(super) async fn forward(
    mut stream: MessageStream,
    inbound: UnboundedSender<Inbound>,
    peers: Arc<Peers>,
) {
    let claim = voiced();
    let mut senders = SignalSenders::default();
    while let Some(Ok(message)) = next(&mut stream).await {
        let header = message.header();
        let Some(path) = header.path().map(ToString::to_string) else {
            continue;
        };
        if senders
            .admit(&peers, &header, std::slice::from_ref(&claim))
            .await
            .is_some()
            && let Ok(text) = message.body().deserialize::<String>()
            && let Some(end) = open(&text)
            && inbound.send(Inbound::Signal(path, end)).is_err()
        {
            return;
        }
    }
}
