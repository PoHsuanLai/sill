//! The loop: keep the published list current from the daemon's `Publish`, and once told to
//! `Serve` answer on the bus and say when the list changes.

use sill_model::workspace_looks::{WorkspaceLooks, WorkspaceLooksCommand, WorkspaceLooksLink};
use tokio::sync::watch;
use zbus::Connection;

use super::object::WorkspacesObject;
use super::serve;
use crate::env::Bus;
use crate::service::ServiceIo;
use crate::workspace_looks::{PATH, WorkspaceLooksService};

pub(crate) async fn run(bus: Bus, io: ServiceIo<WorkspaceLooksService>) {
    let ServiceIo {
        state,
        mut commands,
    } = io;
    let mut seat: Option<Connection> = None;
    while let Some(command) = commands.recv().await {
        match command {
            WorkspaceLooksCommand::Serve if seat.is_none() => seat = claim(&bus, &state).await,
            WorkspaceLooksCommand::Serve => {}
            WorkspaceLooksCommand::Publish(workspaces) => {
                let changed = state.send_if_modified(|published| {
                    let changed = published.workspaces != workspaces;
                    if changed {
                        published.workspaces = workspaces;
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

async fn claim(bus: &Bus, state: &watch::Sender<WorkspaceLooks>) -> Option<Connection> {
    match serve::take(bus, state.subscribe()).await {
        Ok(connection) => {
            state.send_modify(|published| published.link = WorkspaceLooksLink::Serving);
            Some(connection)
        }
        Err(trouble) => {
            state.send_modify(|published| {
                published.link = WorkspaceLooksLink::Unavailable(trouble);
            });
            None
        }
    }
}

/// Send `PropertiesChanged` for `Workspaces`.
async fn announce(connection: &Connection) {
    let Ok(object) = connection
        .object_server()
        .interface::<_, WorkspacesObject>(PATH)
        .await
    else {
        return;
    };
    let emitter = object.signal_emitter();
    if let Err(error) = object.get().await.workspaces_changed(emitter).await {
        eprintln!("sill: workspace looks: announcing the change failed: {error}");
    }
}
