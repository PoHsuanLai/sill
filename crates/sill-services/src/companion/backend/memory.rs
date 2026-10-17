//! The memory timeline, read from memoryd as the shell: newest first, every actor and every app.

use almanac_core::{
    ActorFilter, FactId, FactView, Settlement, SpaceState, SpaceSummary, TimelineFilter,
    TimelinePage, TimelineQuery, TrustFilter,
};
use almanac_dbus::ControlProxy;
use ds_core::vocab::MemoryVerb;
use porter_core::Count;
use prov::{ConfirmId, ConfirmReceipt, InputProof, SpaceId, UnixSeconds};
use sill_launcher::Generation;
use sill_model::companion::CompanionEvent;
use zbus::Connection;

use super::text::{from_json, text, to_json};

/// How many entries one read asks for.
const PAGE: u32 = 40;

/// The newest page of the timeline and what waits for the person, as events for `generation`:
/// the waiting list first, so it is in when the timeline's answer ends the wait.
pub(super) async fn timeline(
    connection: Connection,
    generation: Generation,
) -> Vec<CompanionEvent> {
    let (waiting, timeline) = tokio::join!(pending(&connection), page(&connection));
    let waiting = match waiting {
        Ok(rows) => CompanionEvent::WaitingLoaded(generation, rows),
        Err(why) => {
            eprintln!("sill: companion: what waits to be remembered: {why}");
            CompanionEvent::WaitingFailed(generation)
        }
    };
    let timeline = match timeline {
        Ok(page) => CompanionEvent::TimelineLoaded(generation, page.entries),
        Err(why) => {
            eprintln!("sill: companion: the memory timeline: {why}");
            CompanionEvent::TimelineFailed(generation)
        }
    };
    vec![waiting, timeline]
}

/// The person's answer about one waiting memory, as the event that says how it went.
pub(super) async fn settle(
    connection: Connection,
    fact: FactId,
    verb: MemoryVerb,
    at: UnixSeconds,
) -> CompanionEvent {
    match settled(&connection, &fact, verb, at).await {
        Ok(()) => CompanionEvent::Settled(fact),
        Err(why) => {
            eprintln!("sill: companion: answering a memory: {why}");
            CompanionEvent::SettleFailed
        }
    }
}

/// What waits for the person in every Space: an assistant's job in another Space (a helper's in
/// "work") asks to remember there. Memory opens a Space on first use, so one not opened yet is
/// asked too; one being deleted is not.
async fn pending(connection: &Connection) -> Result<Vec<FactView>, String> {
    let control = ControlProxy::new(connection).await.map_err(text)?;
    let spaces: Vec<SpaceSummary> = from_json(&control.spaces().await.map_err(text)?)?;
    let mut waiting = Vec::new();
    for space in spaces
        .iter()
        .filter(|s| !matches!(s.state, SpaceState::Deleting | SpaceState::Gone))
    {
        let reply = control.pending(space.id.as_str()).await.map_err(text)?;
        waiting.extend(from_json::<Vec<FactView>>(&reply)?);
    }
    Ok(waiting)
}

/// Keeps or drops `fact`. Keep carries the shell's own receipt, which covers exactly the fact's
/// own confidentiality, so keeping opens nothing. A fact no longer waiting is already answered.
async fn settled(
    connection: &Connection,
    fact: &FactId,
    verb: MemoryVerb,
    at: UnixSeconds,
) -> Result<(), String> {
    let waiting = pending(connection).await?;
    let Some(view) = waiting.iter().find(|view| &view.fact.id == fact) else {
        return Ok(());
    };
    let settlement = match verb {
        MemoryVerb::Keep => Settlement::Keep(receipt(view, at)?),
        MemoryVerb::Discard => Settlement::Discard,
        _ => return Err("not a way to answer a memory".to_owned()),
    };
    let control = ControlProxy::new(connection).await.map_err(text)?;
    control
        .settle(&to_json(fact)?, &to_json(&settlement)?)
        .await
        .map_err(text)
}

fn receipt(view: &FactView, at: UnixSeconds) -> Result<ConfirmReceipt, String> {
    Ok(ConfirmReceipt {
        id: ConfirmId::parse(&format!("keep-{}-{}", view.fact.id, at.0)).map_err(text)?,
        input: InputProof::ShellCaller,
        at,
        covers: view.fact.label.confidentiality.clone(),
    })
}

async fn page(connection: &Connection) -> Result<TimelinePage, String> {
    let query = TimelineQuery {
        before: None,
        limit: Count(PAGE),
        filter: TimelineFilter {
            actors: ActorFilter::Everyone,
            apps: Vec::new(),
            kinds: Vec::new(),
            trust: TrustFilter::Any,
            range: None,
        },
    };
    let control = ControlProxy::new(connection).await.map_err(text)?;
    let space = SpaceId::desktop();
    let reply = control
        .timeline(space.as_str(), &to_json(&query)?)
        .await
        .map_err(text)?;
    from_json(&reply)
}
