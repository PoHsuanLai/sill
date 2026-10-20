//! The loop: keep the published list current from the daemon's `Publish`, claiming the bus name
//! at the start (unless the [`Env`](crate::env::Env) leaves names alone) and saying when the list
//! changes.

use sill_model::workspace_looks::{WorkspaceLooks, WorkspaceLooksCommand};
use tokio::sync::watch;
use zbus::Connection;

use super::object::WorkspacesObject;
use crate::env::{Bus, NameClaims};
use crate::serve;
use crate::service::ServiceIo;
use crate::workspace_looks::{BUS_NAME, PATH, WorkspaceLooksService};

pub(crate) async fn run(bus: Bus, claims: NameClaims, io: ServiceIo<WorkspaceLooksService>) {
    let ServiceIo {
        state,
        mut commands,
    } = io;
    let seat = match claims {
        NameClaims::Claim => claim(&bus, &state).await,
        NameClaims::Passive => None,
    };
    while let Some(WorkspaceLooksCommand::Publish(workspaces)) = commands.recv().await {
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

async fn claim(bus: &Bus, state: &watch::Sender<WorkspaceLooks>) -> Option<Connection> {
    let claimed = serve::claim(bus, BUS_NAME, async |connection| {
        let object = WorkspacesObject {
            state: state.subscribe(),
        };
        connection
            .object_server()
            .at(PATH, object)
            .await
            .map(|_| ())
            .map_err(serve::refused)
    })
    .await;
    let (link, seat) = serve::held(claimed);
    state.send_modify(|published| published.link = link);
    seat
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
