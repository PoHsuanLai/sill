//! The IPC server: an accept loop on a std thread that hands every connection to a thread of its
//! own, which reads one request, forwards it to the host thread and writes the reply, or
//! `Failed(Timeout)` after [`REPLY_TIMEOUT`].
//!
//! No client can hold another up: a connection that has not sent its whole line within
//! [`READ_DEADLINE`] is cut, at most [`MAX_CONNECTIONS`] are served at once (more are closed
//! unanswered), and only processes of the same user are served at all.
//!
//! The accept loop ends when asked ([`Serving::stop`]): a flag the loop reads after every accept,
//! and a knock on the socket to wake the accept that is blocking. A daemon whose lock file was
//! removed cannot be knocked (its path may now be another daemon's), so it does not try and the
//! loop ends with the process.
//!
//! The accept loop and the per-connection read/dispatch/write are generic over how a request is
//! answered ([`spawn_with`]) so `crates/sill/tests/it/ipc.rs` can drive them with a fake
//! dispatcher instead of a running [`HostHandle`] and [`shell_host::Host`], which need a live
//! Wayland display. [`spawn`] is the real dispatcher, over `ShellMsg::Ipc`.

use std::io::{self, BufReader, Read};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use latchkey::lock::Standing;
use latchkey::{Agent, Stream};
use shell_host::HostHandle;
use sill_ipc::error::IpcError;
use sill_ipc::line::{read_message, write_message};
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};

use super::door::Door;
use crate::daemon::msg::ShellMsg;

/// How long a client waits for the host thread's answer.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// How long a client has to send its whole request line: the command line writes it at once, so
/// anything slower is stuck or hostile, and must not hold a connection.
pub const READ_DEADLINE: Duration = Duration::from_secs(1);

/// How long writing the reply may take before the connection is dropped (a client that stopped
/// reading).
const WRITE_DEADLINE: Duration = Duration::from_secs(2);

/// The most connections served at once. The command line opens one at a time, so a handful is
/// plenty; past it a connection is closed unanswered rather than given a thread.
pub const MAX_CONNECTIONS: usize = 16;

/// Serve `door` until [`Serving::stop`] or until the process exits with the host.
pub fn spawn(door: Door, host: HostHandle<ShellMsg>) -> Serving {
    spawn_with(door, move |request, reply| {
        dispatch_via_host(&host, request, reply)
    })
}

/// A running accept loop, and what ends it.
#[derive(Debug)]
pub struct Serving {
    door: Door,
    end: mpsc::Sender<()>,
    thread: JoinHandle<()>,
}

impl Serving {
    /// Ask the accept loop to end, and wake it if the socket can still be reached: it is knocked
    /// on through `agent` only while the lock file is still this daemon's, because otherwise the
    /// path is no longer ours to knock on. The thread's handle is returned for a caller that wants
    /// to wait for it; a displaced daemon's loop ends only with the process, so waiting on it
    /// there would wait for ever.
    pub fn stop(self, agent: &Agent) -> JoinHandle<()> {
        let _ = self.end.send(());
        if matches!(self.door.standing(), Standing::Ours) {
            let _ = agent.connect();
        }
        self.thread
    }
}

/// [`spawn`], but answering every request through `dispatch` instead of a running host. `dispatch`
/// is handed the request and a reply channel; it may answer synchronously or hand the channel to
/// another thread, exactly as [`dispatch_via_host`] hands it to [`HostHandle::send`]. Each
/// connection gets its own clone.
pub fn spawn_with(
    door: impl Into<Door>,
    dispatch: impl Fn(Request, mpsc::SyncSender<Response>) + Clone + Send + 'static,
) -> Serving {
    let door: Door = door.into();
    let (end, ended) = mpsc::channel();
    let accepting = door.clone();
    let thread = std::thread::spawn(move || {
        let live = Arc::new(AtomicUsize::new(0));
        loop {
            let accepted = accepting.accept();
            // Read after the accept returns: the knock that wakes a blocked accept arrives as a
            // connection like any other, and is dropped here unanswered.
            if ended.try_recv().is_ok() {
                break;
            }
            match accepted {
                Ok(stream) => serve_off_loop(stream, &dispatch, &live),
                Err(error) => eprintln!("sill: ipc accept: {error}"),
            }
        }
    });
    Serving { door, end, thread }
}

/// Give `stream` a thread of its own, unless [`MAX_CONNECTIONS`] are being served.
fn serve_off_loop(
    stream: Stream,
    dispatch: &(impl Fn(Request, mpsc::SyncSender<Response>) + Clone + Send + 'static),
    live: &Arc<AtomicUsize>,
) {
    let Some(slot) = Slot::take(live) else {
        return;
    };
    let dispatch = dispatch.clone();
    let spawned = std::thread::Builder::new()
        .name("sill-ipc-conn".to_owned())
        .spawn(move || {
            let _slot = slot;
            match own_user_stream(stream) {
                Ok(stream) => serve_one_with(stream, &dispatch),
                Err(error) => eprintln!("sill: ipc connection refused: {error}"),
            }
        });
    if let Err(error) = spawned {
        eprintln!("sill: ipc connection thread: {error}");
    }
}

/// One of the [`MAX_CONNECTIONS`] places; gives itself back when dropped.
struct Slot(Arc<AtomicUsize>);

impl Slot {
    fn take(live: &Arc<AtomicUsize>) -> Option<Slot> {
        if live.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            live.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Slot(Arc::clone(live)))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// `stream` as a plain Unix socket with its write deadline set, if its peer is this user's.
fn own_user_stream(stream: Stream) -> io::Result<UnixStream> {
    let socket = UnixStream::from(latchkey::into_fd(stream)?);
    let peer = rustix::net::sockopt::socket_peercred(&socket)?;
    if peer.uid != rustix::process::getuid() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the peer is another user's process",
        ));
    }
    socket.set_write_timeout(Some(WRITE_DEADLINE))?;
    Ok(socket)
}

/// A reader that gives up at one fixed instant, however slowly the peer drips bytes in.
struct ReadUntil<'a> {
    socket: &'a UnixStream,
    deadline: Instant,
}

impl Read for ReadUntil<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| io::Error::from(io::ErrorKind::TimedOut))?;
        self.socket.set_read_timeout(Some(left))?;
        let mut socket: &UnixStream = self.socket;
        socket.read(buf)
    }
}

/// Read one request from `socket` (within [`READ_DEADLINE`]), hand it to `dispatch`, write the
/// reply.
fn serve_one_with(mut socket: UnixStream, dispatch: &impl Fn(Request, mpsc::SyncSender<Response>)) {
    let response = match read_request(&socket) {
        Ok(request) => answer(dispatch, request),
        Err(error) => malformed(&error),
    };
    let _ = write_message(&mut socket, &response);
}

fn read_request(socket: &UnixStream) -> Result<Request, IpcError> {
    let reader = ReadUntil {
        socket,
        deadline: Instant::now() + READ_DEADLINE,
    };
    read_message(&mut BufReader::new(reader))
}

/// Dispatch `request` and wait up to [`REPLY_TIMEOUT`] for the answer.
fn answer(dispatch: &impl Fn(Request, mpsc::SyncSender<Response>), request: Request) -> Response {
    let (reply, answer) = mpsc::sync_channel(1);
    dispatch(request, reply);
    answer
        .recv_timeout(REPLY_TIMEOUT)
        .unwrap_or_else(|_| timeout())
}

/// Post `request` to the host's event loop, to be answered as `ShellMsg::Ipc`. A host already
/// gone (the reply channel is dropped without sending) surfaces as `answer`'s timeout, same as
/// any other silence.
fn dispatch_via_host(
    host: &HostHandle<ShellMsg>,
    request: Request,
    reply: mpsc::SyncSender<Response>,
) {
    let _ = host.send(ShellMsg::Ipc { request, reply });
}

fn timeout() -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Timeout,
        message: format!("no reply within {}s", REPLY_TIMEOUT.as_secs()),
    })
}

fn malformed(error: &IpcError) -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Internal,
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_CONNECTIONS, Slot, answer, malformed, timeout};
    use sill_ipc::error::IpcError;
    use sill_ipc::request::Request;
    use sill_ipc::response::{Failure, FailureKind, Response};
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn answer_falls_back_to_timeout_when_nothing_replies() {
        assert_eq!(answer(&|_req, _reply| {}, Request::Ping), timeout());
    }

    #[test]
    fn malformed_carries_the_parse_error() {
        let error = IpcError::TooLong { limit: 8 };
        assert_eq!(
            malformed(&error),
            Response::Failed(Failure {
                kind: FailureKind::Internal,
                message: error.to_string(),
            })
        );
    }

    #[test]
    fn a_full_house_refuses_the_next_connection_until_one_leaves() {
        let live = Arc::new(AtomicUsize::new(0));
        let held: Vec<Slot> = (0..MAX_CONNECTIONS)
            .map(|_| Slot::take(&live).unwrap_or_else(|| panic!("a free place")))
            .collect();
        assert!(Slot::take(&live).is_none(), "the {MAX_CONNECTIONS}+1st");
        drop(held);
        assert!(Slot::take(&live).is_some(), "a place came back");
    }
}
