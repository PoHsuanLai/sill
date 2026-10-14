//! The standard-protocol arm's thread: the wire client on the connection the service's thread
//! was given, a calloop loop over its socket and the command channel, and the projected state
//! published after every dispatch.

use std::io;

use desktop_client::Desktop;
use desktop_client::client::DesktopClient;
use sill_model::windows::{Toplevels, WindowModel, WlCommand};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::{EventLoop, Interest, Mode, PostAction, channel};
use tokio::sync::mpsc::UnboundedReceiver;
use wayland_client::Connection;
use wayland_client::backend::WaylandError;

use super::command::{Refusal, desktop_of, wire_of};
use super::commands::{forward_commands, park_commands};
use super::publish::Publish;
use super::{toplevels, workspaces};

/// The thread's body: follow the compositor until every `Control` is dropped or the connection
/// ends, then publish empty values.
pub(super) fn run(
    connection: Connection,
    publish: Publish,
    commands: UnboundedReceiver<WlCommand>,
) {
    let client = match DesktopClient::new(&connection) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("sill: desktop_wl: {error}; toplevels and workspaces stay empty");
            park_commands(commands);
            // Held until now so the receivers read empty values, not a closed channel.
            drop(publish);
            return;
        }
    };
    let mut wlr = Wlr {
        client,
        publish,
        recency: Toplevels::default(),
    };
    wlr.sync();
    follow(wlr, commands);
}

/// What the loop dispatches into.
struct Wlr {
    client: DesktopClient,
    publish: Publish,
    /// Windows in recency order; the protocols give none, so the projection keeps it.
    recency: Toplevels,
}

impl Wlr {
    /// Publish the client's state, projected.
    fn sync(&mut self) {
        let _events_are_reduced_into_the_state = self.client.drain_events();
        let state = self.client.state();
        let recency = toplevels::project(std::mem::take(&mut self.recency), state);
        self.publish.replace(WindowModel {
            workspaces: workspaces::project(state),
            toplevels: recency.clone(),
        });
        self.recency = recency;
    }

    /// Read what the compositor sent, dispatch it and publish.
    fn read(&mut self) -> io::Result<()> {
        if let Some(guard) = self.client.connection().prepare_read() {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(io::Error::other(error)),
            }
        }
        self.client.dispatch_pending().map_err(io::Error::other)?;
        self.sync();
        Ok(())
    }

    /// Carry out `command`; one the protocols cannot carry, or that names something gone, is
    /// dropped (the published state is the answer).
    fn execute(&mut self, command: WlCommand) {
        let state = self.client.state();
        let wire = desktop_of(&command, state)
            .ok_or(Refusal::Unsupported)
            .and_then(|command| wire_of(&command, state));
        match wire {
            Ok(requests) => {
                for request in requests {
                    if let Err(error) = self.client.send(request) {
                        eprintln!("sill: desktop_wl: {error}");
                        break;
                    }
                }
            }
            Err(Refusal::Unsupported) => {}
            Err(refusal) => eprintln!("sill: desktop_wl: {refusal}"),
        }
    }
}

fn follow(mut wlr: Wlr, commands: UnboundedReceiver<WlCommand>) {
    let mut event_loop = match EventLoop::<Wlr>::try_new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            eprintln!("sill: desktop_wl: {error}; toplevels and workspaces are now empty");
            wlr.publish.clear();
            park_commands(commands);
            return;
        }
    };
    let (to_loop, from_forwarder) = channel::channel();
    let forwarder = match std::thread::Builder::new()
        .name("desktop_wl-commands".to_owned())
        .spawn(move || forward_commands(commands, to_loop))
    {
        Ok(forwarder) => forwarder,
        Err(error) => {
            eprintln!("sill: desktop_wl: no command thread: {error}; commands are refused");
            return;
        }
    };
    if let Err(error) = drive(&mut event_loop, &mut wlr, from_forwarder) {
        eprintln!("sill: desktop_wl: {error}; toplevels and workspaces are now empty");
    }
    wlr.publish.clear();
    // The watch senders live in `wlr`: keep them until the forwarder has seen every Control go,
    // so receivers read empty values rather than a closed channel.
    let _finished = forwarder.join();
}

/// Dispatch the socket and the command channel into `wlr` until the channel closes or the
/// connection fails.
fn drive(
    event_loop: &mut EventLoop<'static, Wlr>,
    wlr: &mut Wlr,
    commands: channel::Channel<WlCommand>,
) -> Result<(), calloop::Error> {
    let signal = event_loop.get_signal();
    let handle = event_loop.handle();
    handle
        .insert_source(socket_source(wlr)?, |_, _, wlr: &mut Wlr| {
            wlr.read().map(|()| PostAction::Continue)
        })
        .map_err(|error| error.error)?;
    handle
        .insert_source(commands, move |event, _, wlr: &mut Wlr| match event {
            channel::Event::Msg(command) => {
                wlr.execute(command);
                wlr.sync();
            }
            channel::Event::Closed => signal.stop(),
        })
        .map_err(|error| error.error)?;
    event_loop.run(None, wlr, |_| {})
}

/// The connection's socket as a loop source, on a duplicate of its descriptor.
fn socket_source(wlr: &Wlr) -> io::Result<Generic<std::os::fd::OwnedFd, io::Error>> {
    let fd = wlr
        .client
        .connection()
        .backend()
        .poll_fd()
        .try_clone_to_owned()?;
    Ok(Generic::new(fd, Interest::READ, Mode::Level))
}
