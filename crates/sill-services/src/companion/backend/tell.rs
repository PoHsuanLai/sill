//! Saying something to a subagent: the router delivers the person's words as a request
//! (`Intents1.Message.Send`), then the companion is told so its roster line shows them at once
//! (`Companion1.Told`). The words are recorded as the person's turn first, the same way an ask
//! is, because `Told` carries that recorded turn.

use docket_client::{DbusTransport, Intents};
use docket_core::{
    ContextKeep, DraftPart, Keep, MessageDraft, TurnIn, TurnSource, TurnVia, UserTurn,
};
use docket_dbus::CompanionProxy;
use docket_vocab::Origin;
use docket_wire::{SessionOpen, SessionOpened};
use prov::{Address, AgentRef, MessageKind, MessageText, SpaceId, UnixSeconds};
use zbus::Connection;

use super::text::{from_json, to_json};
use crate::call_error::CallError;

/// Say `words` to `agent` in `space`, as the person.
pub(super) async fn tell(
    connection: &Connection,
    agent: AgentRef,
    space: SpaceId,
    words: String,
    at: UnixSeconds,
) -> Result<(), CallError> {
    let companion = CompanionProxy::new(connection).await?;
    let open = SessionOpen {
        space: space.clone(),
        agent: AgentRef::Companion,
        parent: None,
        cwd: None,
        started_from: None,
        external: None,
        helper: None,
        acts_for: None,
    };
    let opened: SessionOpened = from_json(&companion.open(&to_json(&open)?).await?)?;
    let intents = Intents::over(DbusTransport::new(connection.clone()));
    let turn = TurnIn {
        text: words.clone(),
        origin: Origin::Launcher,
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Dropped,
        },
        via: TurnVia::Typed,
        mode: None,
    };
    let id = intents.session_turn(opened.session.clone(), turn).await?;
    let draft = MessageDraft {
        to: Address {
            agent: agent.clone(),
            space: space.clone(),
        },
        thread: None,
        in_reply_to: None,
        kind: MessageKind::Request,
        parts: vec![DraftPart::Text(MessageText::new(words.clone()))],
    };
    let sent = intents.send(opened.session.clone(), draft).await;
    let told = match sent {
        Ok(_) => {
            let recorded = UserTurn {
                id,
                text: words,
                at,
                from: TurnSource::Launcher,
                via: TurnVia::Typed,
                mode: None,
            };
            companion
                .told(&to_json(&agent)?, space.as_str(), &to_json(&recorded)?)
                .await
                .map_err(CallError::from)
        }
        Err(error) => Err(error.into()),
    };
    // The session was only for this turn; closing it is best effort.
    let _ = companion.close(opened.session.as_str()).await;
    told
}
