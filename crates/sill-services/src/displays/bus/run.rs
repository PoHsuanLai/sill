//! The loop: claim the name at the start (unless the environment leaves names alone), then send
//! `Changed` each time the listed levels differ from the last ones sent.

use std::sync::Arc;

use sill_model::displays::wire_of;
use sill_model::serve::ServeLink;
use zbus::Connection;

use super::object::DisplaysObject;
use super::{BUS, DisplaysBusLinks, DisplaysBusService, PATH};
use crate::callers::CallerSource;
use crate::env::{Bus, NameClaims};
use crate::serve;
use crate::service::ServiceIo;

pub(super) async fn run(
    bus: Bus,
    claims: NameClaims,
    callers: Arc<dyn CallerSource>,
    links: DisplaysBusLinks,
    io: ServiceIo<DisplaysBusService>,
) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let callers = callers.as_ref();
    let DisplaysBusLinks {
        mut displays,
        control,
    } = links;
    let object_displays = displays.clone();
    let claimed = match claims {
        NameClaims::Claim => {
            serve::claim(&bus, BUS, async move |connection| {
                let object = DisplaysObject {
                    displays: object_displays,
                    control,
                    peers: callers.peers_on(connection.clone()),
                };
                connection
                    .object_server()
                    .at(PATH, object)
                    .await
                    .map(|_| ())
                    .map_err(serve::refused)
            })
            .await
        }
        NameClaims::Passive => {
            // Held until the service is dropped, so its handle keeps answering sends.
            while commands.recv().await.is_some() {}
            return;
        }
    };
    // Taken before the name is announced as ours, so a change after it is never missed.
    let mut sent = wire_of(&displays.borrow_and_update().levels());
    let (link, connection) = serve::held(claimed);
    tx.send_replace(link);
    let Some(connection) = connection else {
        while commands.recv().await.is_some() {}
        return;
    };
    loop {
        tokio::select! {
            command = commands.recv() => if command.is_none() { break },
            changed = displays.changed() => {
                if changed.is_err() {
                    break;
                }
                let now = wire_of(&displays.borrow_and_update().levels());
                if now != sent {
                    announce(&connection, &now).await;
                    sent = now;
                }
            }
        }
    }
    tx.send_replace(ServeLink::Passive);
}

async fn announce(connection: &Connection, levels: &[(String, u32)]) {
    let sent = connection
        .emit_signal(
            None::<&str>,
            PATH,
            "org.quire.Displays1",
            "Changed",
            &(levels,),
        )
        .await;
    if let Err(error) = sent {
        eprintln!("sill: displays: telling Settings the brightness changed failed: {error}");
    }
}
