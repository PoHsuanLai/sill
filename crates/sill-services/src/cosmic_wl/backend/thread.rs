//! The `cosmic_wl` thread: its own Wayland connection (a protocol object cannot cross
//! connections, and shell-host's connection is the host's), a calloop loop over it, and the
//! command channel. The compositor's globals pick the arm (`backend_of`): the standard
//! protocols and the desktop's own run on `desktop_wl`; COSMIC's own run here. On a compositor
//! with none of them it parks on the command channel instead, dropping commands, until every
//! `Control` is gone.

use cosmic_client_toolkit::sctk::reexports::calloop;
use cosmic_client_toolkit::sctk::reexports::calloop::channel::Event;
use cosmic_client_toolkit::sctk::reexports::calloop::{EventLoop, channel};
use cosmic_client_toolkit::sctk::reexports::calloop_wayland_source::WaylandSource;
use cosmic_client_toolkit::wayland_client::globals::{
    GlobalError, GlobalList, registry_queue_init,
};
use cosmic_client_toolkit::wayland_client::{Connection, EventQueue};
use sill_model::cosmic_wl::WlCommand;
use sill_model::desktop::DesktopBackend;
use tokio::sync::mpsc::UnboundedReceiver;

use super::commands::{forward_commands, park_commands};
use super::execute::execute;
use super::publish::Publish;
use super::state::{Offer, WlState};
use crate::desktop_wl::{self, backend_of};
use crate::env::{WaylandRoute, WaylandUnreached};

/// Why the thread is not following a compositor.
#[derive(Debug, thiserror::Error)]
enum Refusal {
    #[error("{0}")]
    Unreached(#[from] WaylandUnreached),
    #[error("the registry: {0}")]
    Registry(#[from] GlobalError),
    #[error("the compositor offers neither ext-foreign-toplevel-list nor ext-workspace")]
    NoCosmic,
    #[error("the event loop: {0}")]
    Loop(#[from] calloop::Error),
}

/// The thread's body.
pub(crate) fn run(
    endpoint: WaylandRoute,
    publish: Publish,
    commands: UnboundedReceiver<WlCommand>,
) {
    match open(&endpoint, publish) {
        Ok(Opened::Cosmic(session)) => follow(*session, commands),
        Ok(Opened::Wlr(connection, publish)) => desktop_wl::run(connection, publish, commands),
        Err((refusal, publish)) => {
            eprintln!("sill: cosmic_wl: {refusal}; toplevels and workspaces stay empty");
            park_commands(commands);
            // Held until now so the receivers read empty values, not a closed channel.
            drop(publish);
        }
    }
}

/// A bound state and the loop that dispatches into it.
struct Session {
    event_loop: EventLoop<'static, WlState>,
    state: WlState,
}

/// The arm the compositor's globals select, ready to run.
enum Opened {
    Cosmic(Box<Session>),
    /// The standard protocols or the desktop's own: the connection, which the arm binds itself.
    Wlr(Connection, Publish),
}

fn open(endpoint: &WaylandRoute, publish: Publish) -> Result<Opened, (Refusal, Publish)> {
    let (conn, globals, queue) = match connect(endpoint) {
        Ok(parts) => parts,
        Err(refusal) => return Err((refusal, publish)),
    };
    match backend_of(&desktop_wl::offer_of_registry(&globals)) {
        // The desktop's own compositor serves the toplevels and workspaces on this connection
        // as the standard arm reads them (the client projects the Quire window state onto the
        // toplevels' flags); its shell role lives on the shell host's connection
        // (`desktop_wl::quire`).
        DesktopBackend::Quire | DesktopBackend::Wlr => Ok(Opened::Wlr(conn, publish)),
        DesktopBackend::Cosmic => open_cosmic(conn, globals, queue, publish)
            .map(|session| Opened::Cosmic(Box::new(session))),
    }
}

fn open_cosmic(
    conn: Connection,
    globals: GlobalList,
    queue: EventQueue<WlState>,
    publish: Publish,
) -> Result<Session, (Refusal, Publish)> {
    let state = WlState::bind(&globals, &queue.handle(), publish);
    if state.offer() == Offer::Nothing {
        return Err((Refusal::NoCosmic, state.into_publish()));
    }
    let event_loop = match EventLoop::<WlState>::try_new() {
        Ok(event_loop) => event_loop,
        Err(error) => return Err((error.into(), state.into_publish())),
    };
    if let Err(error) = WaylandSource::new(conn, queue).insert(event_loop.handle()) {
        return Err((error.error.into(), state.into_publish()));
    }
    Ok(Session { event_loop, state })
}

type Connected = (Connection, GlobalList, EventQueue<WlState>);

fn connect(endpoint: &WaylandRoute) -> Result<Connected, Refusal> {
    let conn = endpoint.connect()?;
    let (globals, queue) = registry_queue_init::<WlState>(&conn)?;
    Ok((conn, globals, queue))
}

/// Run the loop until every `Control` is dropped (or the compositor goes away, after which
/// the channels are emptied and commands dropped as on a compositor without the COSMIC
/// protocols).
fn follow(session: Session, commands: UnboundedReceiver<WlCommand>) {
    let Session {
        mut event_loop,
        mut state,
    } = session;
    let (to_loop, from_forwarder) = channel::channel();
    let forwarder = match std::thread::Builder::new()
        .name("cosmic_wl-commands".to_owned())
        .spawn(move || forward_commands(commands, to_loop))
    {
        Ok(forwarder) => forwarder,
        Err(error) => {
            eprintln!("sill: cosmic_wl: no command thread: {error}; commands are refused");
            return;
        }
    };
    let signal = event_loop.get_signal();
    let ended = event_loop
        .handle()
        .insert_source(
            from_forwarder,
            move |event, _, state: &mut WlState| match event {
                Event::Msg(command) => execute(state, command),
                Event::Closed => signal.stop(),
            },
        )
        .map_err(|error| error.error)
        .and_then(|_| event_loop.run(None, &mut state, |_| {}));
    if let Err(error) = ended {
        eprintln!("sill: cosmic_wl: {error}; toplevels and workspaces are now empty");
    }
    state.publish.clear();
    // The watch senders live in `state`: keep them until the forwarder has seen every Control
    // go, so receivers read empty values rather than a closed channel.
    let _finished = forwarder.join();
}
