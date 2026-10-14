//! The Quire arm running on the shell host's connection: a thread that binds the desktop
//! client (with the shell role) beside the host on the same socket, follows the compositor,
//! publishes the desktop's view and the shell protocol's notes, and carries out commands and
//! surface registrations sent to it.
//!
//! The host reads the socket on its own thread. Both threads read through the connection's
//! cooperative read guards, so whichever reads first dispatches into both queues; events the
//! host's read leaves in this queue do not wake this thread, so a short tick dispatches them.

use std::io;
use std::time::Duration;

use casement_protocols::vocab::ShellRole;
use desktop_client::DesktopConfig;
use desktop_client::client::DesktopClient;
use sill_model::desktop::{DesktopCommand, DesktopView, ShellNote};
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{
    self, EventLoop, Interest, Mode, PostAction, channel,
};
use tokio::sync::{mpsc, watch};
use wayland_client::backend::WaylandError;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy};

use super::arm::{NoteSink, QuireArm};

/// How often events another thread's read left in this queue are dispatched.
const TICK: Duration = Duration::from_millis(20);

/// One thing sent to the arm's thread.
#[derive(Debug)]
pub enum ShellCall {
    /// Carry out a command.
    Command(DesktopCommand),
    /// `surface` is the shell's `role` on `output`: register it with the client and tell the
    /// compositor. Sent again after a hidden surface is shown (it has a new `wl_surface`).
    Register {
        /// What the surface is.
        role: ShellRole,
        /// The output it is on.
        output: String,
        /// The host's `wl_surface`.
        surface: WlSurface,
    },
}

/// The arm's thread has ended (the compositor went away or the client could not bind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the desktop arm is not running")]
pub struct ShellGone;

/// Why the thread did not start.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// The surface's connection is already closed.
    #[error("the surface's connection is gone")]
    Connection,
    /// The thread could not be made.
    #[error("starting the thread: {0}")]
    Thread(#[from] io::Error),
}

/// The handle to a running arm.
#[derive(Debug)]
pub struct ShellArm {
    /// The desktop as the shell sees it, replaced after every change.
    pub view: watch::Receiver<DesktopView>,
    notes: Option<mpsc::UnboundedReceiver<ShellNote>>,
    calls: channel::Sender<ShellCall>,
}

impl ShellArm {
    /// The receiver of the shell protocol's notes; once.
    pub fn take_notes(&mut self) -> Option<mpsc::UnboundedReceiver<ShellNote>> {
        self.notes.take()
    }

    /// Carry out `command` on the arm's thread.
    pub fn command(&self, command: DesktopCommand) -> Result<(), ShellGone> {
        self.call(ShellCall::Command(command))
    }

    /// Register a surface of the host with the arm.
    pub fn register(
        &self,
        role: ShellRole,
        output: impl Into<String>,
        surface: WlSurface,
    ) -> Result<(), ShellGone> {
        self.call(ShellCall::Register {
            role,
            output: output.into(),
            surface,
        })
    }

    fn call(&self, call: ShellCall) -> Result<(), ShellGone> {
        self.calls.send(call).map_err(|_| ShellGone)
    }
}

/// Start the arm on the connection `surface` lives on: the shell host's, which the surface
/// came from.
pub fn spawn_shell_arm(surface: &WlSurface) -> Result<ShellArm, SpawnError> {
    let connection = surface
        .backend()
        .upgrade()
        .map(Connection::from_backend)
        .ok_or(SpawnError::Connection)?;
    let (view_tx, view) = watch::channel(DesktopView::default());
    let (notes_tx, notes) = mpsc::unbounded_channel();
    let (calls, from_callers) = channel::channel();
    std::thread::Builder::new()
        .name("desktop_quire".to_owned())
        .spawn(move || run(connection, view_tx, notes_tx, from_callers))?;
    Ok(ShellArm {
        view,
        notes: Some(notes),
        calls,
    })
}

/// Notes into a channel.
struct ChannelSink(mpsc::UnboundedSender<ShellNote>);

impl NoteSink for ChannelSink {
    fn note(&mut self, note: ShellNote) {
        // A dropped receiver means nobody follows the notes any more.
        let _nobody_follows = self.0.send(note);
    }
}

/// What the loop dispatches into.
struct Following {
    arm: QuireArm<DesktopClient, ChannelSink>,
    view: watch::Sender<DesktopView>,
}

impl Following {
    /// Publish the desktop and hand out the notes.
    fn publish(&mut self) {
        let view = self.arm.sync();
        self.view.send_replace(view);
    }

    /// Dispatch what this queue holds; publish when something arrived.
    fn pump(&mut self) -> io::Result<()> {
        let dispatched = self
            .arm
            .desktop_mut()
            .dispatch_pending()
            .map_err(io::Error::other)?;
        if dispatched > 0 {
            self.publish();
        }
        Ok(())
    }

    /// Read what the compositor sent, dispatch it and publish.
    fn read(&mut self) -> io::Result<()> {
        if let Some(guard) = self.arm.desktop_mut().connection().prepare_read() {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(io::Error::other(error)),
            }
        }
        self.pump()
    }

    fn call(&mut self, call: ShellCall) {
        match call {
            ShellCall::Command(command) => {
                if let Err(error) = self.arm.execute(&command) {
                    eprintln!("sill: desktop_quire: {error}");
                }
            }
            ShellCall::Register {
                role,
                output,
                surface,
            } => {
                let key = self.arm.desktop_mut().register_surface(&surface);
                if let Err(error) = self.arm.assign(role, output, key) {
                    eprintln!("sill: desktop_quire: {error}");
                }
            }
        }
        self.publish();
    }
}

fn run(
    connection: Connection,
    view: watch::Sender<DesktopView>,
    notes: mpsc::UnboundedSender<ShellNote>,
    calls: channel::Channel<ShellCall>,
) {
    let client = match DesktopClient::with_config(&connection, DesktopConfig::holding_shell()) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("sill: desktop_quire: {error}; the desktop view stays empty");
            return;
        }
    };
    let mut following = Following {
        arm: QuireArm::new(client, ChannelSink(notes)),
        view,
    };
    following.publish();
    if let Err(error) = drive(&mut following, calls) {
        eprintln!("sill: desktop_quire: {error}; the desktop view stops here");
    }
}

/// Dispatch the socket, the tick and the calls into `following` until every handle is dropped
/// or the connection fails.
fn drive(
    following: &mut Following,
    calls: channel::Channel<ShellCall>,
) -> Result<(), calloop::Error> {
    let mut event_loop = EventLoop::<Following>::try_new()?;
    let signal = event_loop.get_signal();
    let handle = event_loop.handle();
    let fd = following
        .arm
        .desktop_mut()
        .connection()
        .backend()
        .poll_fd()
        .try_clone_to_owned()?;
    handle
        .insert_source(
            Generic::new(fd, Interest::READ, Mode::Level),
            |_, _, following: &mut Following| following.read().map(|()| PostAction::Continue),
        )
        .map_err(|error| error.error)?;
    handle
        .insert_source(
            Timer::from_duration(TICK),
            |_, _, following: &mut Following| {
                if let Err(error) = following.pump() {
                    eprintln!("sill: desktop_quire: {error}");
                }
                TimeoutAction::ToDuration(TICK)
            },
        )
        .map_err(|error| error.error)?;
    handle
        .insert_source(
            calls,
            move |event, _, following: &mut Following| match event {
                channel::Event::Msg(call) => following.call(call),
                channel::Event::Closed => signal.stop(),
            },
        )
        .map_err(|error| error.error)?;
    event_loop.run(None, following, |_| {})
}
