//! The `cosmic_wl` thread: its own Wayland connection (a protocol object cannot cross
//! connections, and shell-host's connection is the host's), a calloop loop over it, and the
//! command channel. On a compositor without the COSMIC protocols it parks on the command
//! channel instead, dropping commands, until every `Control` is gone.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use cosmic_client_toolkit::sctk::reexports::calloop::channel::{self, Event};
use cosmic_client_toolkit::sctk::reexports::calloop::{self, EventLoop};
use cosmic_client_toolkit::sctk::reexports::calloop_wayland_source::WaylandSource;
use cosmic_client_toolkit::wayland_client::globals::{
    GlobalError, GlobalList, registry_queue_init,
};
use cosmic_client_toolkit::wayland_client::{ConnectError, Connection, EventQueue};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::cosmic_wl::command::WlCommand;
use crate::cosmic_wl::execute::execute;
use crate::cosmic_wl::publish::Publish;
use crate::cosmic_wl::state::{Offer, WlState};

/// Which compositor to connect to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Endpoint {
    /// `$WAYLAND_DISPLAY` (or `$WAYLAND_SOCKET`), as every client does.
    Environment,
    /// A socket by path: the nested compositor in the live test.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "only the live tests name a socket")
    )]
    Socket(PathBuf),
}

/// Why the thread is not following a compositor.
#[derive(Debug, thiserror::Error)]
enum Refusal {
    #[error("no Wayland socket: {0}")]
    Socket(std::io::Error),
    #[error("no Wayland connection: {0}")]
    Connect(#[from] ConnectError),
    #[error("the registry: {0}")]
    Registry(#[from] GlobalError),
    #[error("the compositor offers neither ext-foreign-toplevel-list nor ext-workspace")]
    NoCosmic,
    #[error("the event loop: {0}")]
    Loop(#[from] calloop::Error),
}

/// The thread's body.
pub(super) fn run(endpoint: Endpoint, publish: Publish, commands: UnboundedReceiver<WlCommand>) {
    match open(&endpoint, publish) {
        Ok(session) => follow(session, commands),
        Err((refusal, publish)) => {
            eprintln!("sill: cosmic_wl: {refusal}; toplevels and workspaces stay empty");
            park(commands);
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

fn open(endpoint: &Endpoint, publish: Publish) -> Result<Session, (Refusal, Publish)> {
    let (conn, globals, queue) = match connect(endpoint) {
        Ok(parts) => parts,
        Err(refusal) => return Err((refusal, publish)),
    };
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

fn connect(endpoint: &Endpoint) -> Result<Connected, Refusal> {
    let conn = match endpoint {
        Endpoint::Environment => Connection::connect_to_env()?,
        Endpoint::Socket(path) => {
            Connection::from_socket(UnixStream::connect(path).map_err(Refusal::Socket)?)?
        }
    };
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
        .spawn(move || forward(commands, to_loop))
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

/// Hand every command to the loop; once the loop has stopped, drop them.
fn forward(mut commands: UnboundedReceiver<WlCommand>, to_loop: channel::Sender<WlCommand>) {
    while let Some(command) = commands.blocking_recv() {
        let _dropped_after_the_loop_stopped = to_loop.send(command);
    }
}

/// Sleep on the command channel, dropping commands, until every `Control` is gone.
fn park(mut commands: UnboundedReceiver<WlCommand>) {
    while commands.blocking_recv().is_some() {}
}
