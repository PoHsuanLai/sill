//! The memory timeline, read from memoryd as the shell: newest first, every actor and every app.

use almanac_core::{ActorFilter, TimelineFilter, TimelinePage, TimelineQuery, TrustFilter};
use almanac_dbus::ControlProxy;
use porter_core::Count;
use prov::SpaceId;
use sill_launcher::Generation;
use sill_model::companion::CompanionEvent;
use zbus::Connection;

use super::text::{from_json, text, to_json};

/// How many entries one read asks for.
const PAGE: u32 = 40;

/// The newest page of the timeline as an event for `generation`.
pub(super) async fn timeline(connection: Connection, generation: Generation) -> CompanionEvent {
    match page(&connection).await {
        Ok(page) => CompanionEvent::TimelineLoaded(generation, page.entries),
        Err(why) => {
            eprintln!("sill: companion: the memory timeline: {why}");
            CompanionEvent::TimelineFailed(generation)
        }
    }
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
