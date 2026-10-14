//! The windows thread's start: its own Wayland connection (a protocol object cannot cross
//! connections, and shell-host's connection is the host's), then the arm the compositor's
//! globals select (`backend_of`). On a compositor with no window protocols it parks on the
//! command channel instead, dropping commands, until every `Control` is gone.

use sill_model::desktop::DesktopBackend;
use sill_model::windows::WlCommand;
use tokio::sync::mpsc::UnboundedReceiver;
use wayland_client::globals::{GlobalError, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::choose::backend_of;
use super::commands::park_commands;
use super::offer::offer_of_registry;
use super::publish::Publish;
use super::run::run as follow;
use crate::env::{WaylandRoute, WaylandUnreached};

/// Why the thread is not following a compositor.
#[derive(Debug, thiserror::Error)]
enum Refusal {
    #[error("{0}")]
    Unreached(#[from] WaylandUnreached),
    #[error("the registry: {0}")]
    Registry(#[from] GlobalError),
    #[error("the compositor offers no window protocol")]
    NoWindowProtocol,
}

/// The state the registry probe dispatches into: it only reads the list of globals.
struct Probe;

impl Dispatch<WlRegistry, GlobalListContents> for Probe {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// The thread's body.
pub(super) fn run(
    endpoint: WaylandRoute,
    publish: Publish,
    commands: UnboundedReceiver<WlCommand>,
) {
    match open(&endpoint) {
        Ok(connection) => follow(connection, publish, commands),
        Err(refusal) => {
            eprintln!("sill: windows: {refusal}; toplevels and workspaces stay empty");
            park_commands(commands);
            // Held until now so the receivers read empty values, not a closed channel.
            drop(publish);
        }
    }
}

/// The connection, once the compositor's globals select an arm that can follow it.
fn open(endpoint: &WaylandRoute) -> Result<Connection, Refusal> {
    let connection = endpoint.connect()?;
    let (globals, _queue) = registry_queue_init::<Probe>(&connection)?;
    match backend_of(&offer_of_registry(&globals)) {
        // The desktop's own compositor serves the toplevels and workspaces on this connection
        // as the standard arm reads them (the client projects the Quire window state onto the
        // toplevels' flags); its shell role lives on the shell host's connection
        // (`desktop_wl::quire`).
        DesktopBackend::Quire | DesktopBackend::Wlr => Ok(connection),
        DesktopBackend::Bare => Err(Refusal::NoWindowProtocol),
    }
}
