//! The thread that holds the connection to keycap: a blocking `SOCK_SEQPACKET` socket and a wake
//! descriptor in one `poll`, so a request from the service and a packet from the daemon are both
//! heard at once. It reconnects with a growing wait when the socket is gone, and starts every
//! connection by telling keycap the focus and the config it should have.

use std::io;
use std::os::fd::{AsFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use keycap_protocol::{Compat, MAX_PACKET, ToClient, ToDaemon, compat, decode, encode};
use rustix::event::{EventfdFlags, PollFd, PollFlags, eventfd, poll};
use rustix::net::{
    AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType, connect, recv,
    send, socket_with,
};
use rustix::time::Timespec;
use sill_model::keycap::{KeycapEvent, Wants, reconnect_delay};
use tokio::sync::mpsc::UnboundedSender;

use crate::error::ServiceError;

/// What the service asks of the thread.
enum Order {
    Send(ToDaemon),
    Stop,
}

/// The service's end of the thread: requests go in, and the thread is woken to take them. Dropping
/// it stops the thread.
#[derive(Debug)]
pub(crate) struct Link {
    orders: mpsc::Sender<Order>,
    wake: Arc<OwnedFd>,
}

impl Link {
    /// Send `message` to keycap now, or on the next connection (the thread keeps the latest focus
    /// and config and tells a new connection both).
    pub(crate) fn send(&self, message: ToDaemon) {
        self.order(Order::Send(message));
    }

    fn order(&self, order: Order) {
        if self.orders.send(order).is_ok() {
            ring(&self.wake);
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.order(Order::Stop);
    }
}

/// Start the thread against `path`; what happens on the socket comes back as `events`.
pub(crate) fn start(
    path: PathBuf,
    events: UnboundedSender<KeycapEvent>,
) -> Result<Link, ServiceError> {
    let wake = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)
        .map_err(|errno| ServiceError::Thread(errno.into()))?;
    let wake = Arc::new(wake);
    let (orders, inbox) = mpsc::channel();
    let thread_wake = Arc::clone(&wake);
    thread::Builder::new()
        .name("keycap".to_owned())
        .spawn(move || run(&path, &inbox, &thread_wake, &events))
        .map_err(ServiceError::Thread)?;
    Ok(Link { orders, wake })
}

/// How a connection ended.
enum Ended {
    /// The service dropped its [`Link`]: the thread is done.
    Stopped,
    /// There was a daemon, and it greeted us.
    Served,
    /// There was none, or it spoke another protocol version.
    Unserved,
}

/// What a step of the thread means for the connection.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Turn {
    Continue,
    /// The connection is no good any more.
    Lost,
    /// The service is gone: the thread is done.
    Quit,
}

/// The thread's body: connect, serve until the connection ends, wait, again.
fn run(
    path: &Path,
    inbox: &mpsc::Receiver<Order>,
    wake: &OwnedFd,
    events: &UnboundedSender<KeycapEvent>,
) {
    let mut wants = Wants::default();
    let mut previous_wait = None;
    loop {
        let ended = match open(path) {
            Ok(fd) => serve(&fd, inbox, wake, events, &mut wants),
            Err(error) => {
                note_unreached(path, &error);
                Ended::Unserved
            }
        };
        match ended {
            Ended::Stopped => return,
            Ended::Served => previous_wait = None,
            Ended::Unserved => {}
        }
        let wait = reconnect_delay(previous_wait);
        previous_wait = Some(wait);
        if pause(wait, inbox, wake, &mut wants) == Turn::Quit {
            return;
        }
    }
}

/// A connection to `path`.
fn open(path: &Path) -> io::Result<OwnedFd> {
    let fd = socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC,
        None,
    )?;
    connect(&fd, &SocketAddrUnix::new(path)?)?;
    Ok(fd)
}

/// A daemon that is not there is the usual case on a machine without keycap and while it
/// restarts; only the unexpected is worth a line.
fn note_unreached(path: &Path, error: &io::Error) {
    if !matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    ) {
        eprintln!("sill: keycap: {}: {error}", path.display());
    }
}

/// Wait out `wait`, taking requests as they come so the next connection starts up to date.
fn pause(wait: Duration, inbox: &mpsc::Receiver<Order>, wake: &OwnedFd, wants: &mut Wants) -> Turn {
    let until = Instant::now() + wait;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Turn::Continue;
        }
        if wait_for(None, wake, left).wake {
            drain(wake);
            if take_orders(inbox, wants, |_| Turn::Continue) == Turn::Quit {
                return Turn::Quit;
            }
        }
    }
}

/// What a `poll` found ready.
struct Ready {
    socket: bool,
    wake: bool,
}

/// Wait up to `limit` for `socket` (when there is one) or `wake` to be ready.
fn wait_for(socket: Option<&OwnedFd>, wake: &OwnedFd, limit: Duration) -> Ready {
    let interest = PollFlags::IN | PollFlags::HUP | PollFlags::ERR;
    let mut fds: Vec<PollFd<'_>> = socket
        .into_iter()
        .map(|socket| PollFd::new(socket, interest))
        .collect();
    fds.push(PollFd::new(wake, PollFlags::IN));
    let timeout = Timespec {
        tv_sec: i64::try_from(limit.as_secs()).unwrap_or(i64::MAX),
        tv_nsec: i64::from(limit.subsec_nanos()),
    };
    // An interrupted or failed poll reads as nothing ready; the caller polls again.
    let _ = poll(&mut fds, Some(&timeout));
    let ready = |fd: &PollFd<'_>| !fd.revents().is_empty();
    Ready {
        socket: socket.is_some() && fds.first().is_some_and(ready),
        wake: fds.last().is_some_and(ready),
    }
}

/// Tell the thread something is in the inbox.
fn ring(wake: &OwnedFd) {
    // A counter at its limit already has the thread woken.
    let _ = rustix::io::write(wake, &1u64.to_ne_bytes());
}

/// Take the wake-up that was rung, so the next poll waits again.
fn drain(wake: &OwnedFd) {
    let _ = rustix::io::read(wake, &mut [0u8; 8]);
}

/// Fold every waiting request into `wants` and hand each to `deliver`, which says whether the
/// connection took it.
fn take_orders(
    inbox: &mpsc::Receiver<Order>,
    wants: &mut Wants,
    mut deliver: impl FnMut(&ToDaemon) -> Turn,
) -> Turn {
    let mut turn = Turn::Continue;
    for order in inbox.try_iter() {
        match order {
            Order::Stop => return Turn::Quit,
            Order::Send(message) => {
                *wants = std::mem::take(wants).after(&message);
                if deliver(&message) == Turn::Lost {
                    turn = Turn::Lost;
                }
            }
        }
    }
    turn
}

/// Whether keycap has said hello on this connection.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Greeting {
    Awaited,
    Done,
}

/// One connection, from its first packet to its end.
fn serve(
    fd: &OwnedFd,
    inbox: &mpsc::Receiver<Order>,
    wake: &OwnedFd,
    events: &UnboundedSender<KeycapEvent>,
    wants: &mut Wants,
) -> Ended {
    let mut greeting = Greeting::Awaited;
    let mut buffer = vec![0u8; MAX_PACKET];
    let turn = loop {
        let ready = wait_for(Some(fd), wake, IDLE_POLL);
        let mut turn = Turn::Continue;
        if ready.wake {
            drain(wake);
            turn = match greeting {
                Greeting::Awaited => take_orders(inbox, wants, |_| Turn::Continue),
                Greeting::Done => take_orders(inbox, wants, |message| transmit(fd, message)),
            };
        }
        if turn == Turn::Continue && ready.socket {
            turn = match read_packet(fd, &mut buffer) {
                Packet::Message(message) => handle(fd, message, events, wants, &mut greeting),
                Packet::Skipped => Turn::Continue,
                Packet::Closed => Turn::Lost,
            };
        }
        if turn != Turn::Continue {
            break turn;
        }
    };
    match (turn, greeting) {
        (Turn::Quit, _) => Ended::Stopped,
        (_, Greeting::Done) => {
            let _ = events.send(KeycapEvent::Disconnected);
            Ended::Served
        }
        (_, Greeting::Awaited) => Ended::Unserved,
    }
}

/// How long one poll waits when nothing happens; the loop only needs to come round, so it is
/// long.
const IDLE_POLL: Duration = Duration::from_secs(3600);

/// What one read found.
enum Packet {
    Message(ToClient),
    /// Something that is not a message of the protocol.
    Skipped,
    /// The peer is gone.
    Closed,
}

fn read_packet(fd: &OwnedFd, buffer: &mut [u8]) -> Packet {
    match recv(fd.as_fd(), &mut *buffer, RecvFlags::TRUNC) {
        Ok((0, _)) => Packet::Closed,
        Ok((len, full)) if full > len => {
            eprintln!("sill: keycap: a packet of {full} bytes is over the limit");
            Packet::Skipped
        }
        Ok((len, _)) => match decode::<ToClient>(&buffer[..len]) {
            Ok(message) => Packet::Message(message),
            Err(error) => {
                eprintln!("sill: keycap: a packet that is not a message: {error}");
                Packet::Skipped
            }
        },
        Err(errno) if errno == rustix::io::Errno::INTR => Packet::Skipped,
        Err(_) => Packet::Closed,
    }
}

/// Act on one message: tell the service, and greet a daemon that has said hello. `Lost` ends the
/// connection (the daemon speaks another version, or the greeting could not be sent).
fn handle(
    fd: &OwnedFd,
    message: ToClient,
    events: &UnboundedSender<KeycapEvent>,
    wants: &Wants,
    greeting: &mut Greeting,
) -> Turn {
    if let Compat::Incompatible { version } = compat(&message) {
        eprintln!(
            "sill: keycap: the daemon speaks protocol {version}, this sill {}; it is left alone",
            keycap_protocol::PROTOCOL_VERSION
        );
        return Turn::Lost;
    }
    match message {
        ToClient::Hello { status, .. } => {
            let _ = events.send(KeycapEvent::Hello(status));
            match std::mem::replace(greeting, Greeting::Done) {
                Greeting::Awaited => greet(fd, wants),
                Greeting::Done => Turn::Continue,
            }
        }
        ToClient::Status(status) => {
            let _ = events.send(KeycapEvent::Status(status));
            Turn::Continue
        }
        ToClient::ConfigWarnings(warnings) => {
            for warning in warnings {
                eprintln!(
                    "sill: keycap: keycap.toml {}: {}",
                    warning.key, warning.problem
                );
            }
            Turn::Continue
        }
        // The keymap's own answers go to apps that asked for them; this connection never does.
        ToClient::Keymap(_) | ToClient::Registered(_) => Turn::Continue,
    }
}

/// Tell a new connection everything `wants` holds.
fn greet(fd: &OwnedFd, wants: &Wants) -> Turn {
    wants
        .greeting()
        .iter()
        .map(|message| transmit(fd, message))
        .find(|turn| *turn != Turn::Continue)
        .unwrap_or(Turn::Continue)
}

/// Send `message`; `Lost` when the connection is no longer there to take it.
fn transmit(fd: &OwnedFd, message: &ToDaemon) -> Turn {
    match send(fd, &encode(message), SendFlags::NOSIGNAL) {
        Ok(_) => Turn::Continue,
        Err(_) => Turn::Lost,
    }
}
