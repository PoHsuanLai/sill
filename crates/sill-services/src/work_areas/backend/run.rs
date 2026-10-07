//! The loop: keep the published list current from the daemon's `Publish`, and once told to
//! `Serve` answer on the bus and say when the list changes.

use sill_model::work_areas::{WorkAreaCommand, WorkAreaLink, WorkAreas};
use tokio::sync::watch;
use zbus::Connection;

use super::object::OutputsObject;
use super::serve;
use crate::env::Bus;
use crate::service::ServiceIo;
use crate::work_areas::{PATH, WorkAreaService};

pub(crate) async fn run(bus: Bus, io: ServiceIo<WorkAreaService>) {
    let ServiceIo {
        state,
        mut commands,
    } = io;
    let mut seat: Option<Connection> = None;
    while let Some(command) = commands.recv().await {
        match command {
            WorkAreaCommand::Serve if seat.is_none() => seat = claim(&bus, &state).await,
            WorkAreaCommand::Serve => {}
            WorkAreaCommand::Publish(areas) => {
                let changed = state.send_if_modified(|published| {
                    let changed = published.areas != areas;
                    if changed {
                        published.areas = areas;
                    }
                    changed
                });
                if let (true, Some(connection)) = (changed, &seat) {
                    announce(connection).await;
                }
            }
        }
    }
}

async fn claim(bus: &Bus, state: &watch::Sender<WorkAreas>) -> Option<Connection> {
    match serve::take(bus, state.subscribe()).await {
        Ok(connection) => {
            state.send_modify(|published| published.link = WorkAreaLink::Serving);
            Some(connection)
        }
        Err(trouble) => {
            state.send_modify(|published| published.link = WorkAreaLink::Unavailable(trouble));
            None
        }
    }
}

/// Send `PropertiesChanged` for `WorkAreas`.
async fn announce(connection: &Connection) {
    let Ok(object) = connection
        .object_server()
        .interface::<_, OutputsObject>(PATH)
        .await
    else {
        return;
    };
    let emitter = object.signal_emitter();
    if let Err(error) = object.get().await.work_areas_changed(emitter).await {
        eprintln!("sill: work areas: announcing the change failed: {error}");
    }
}
