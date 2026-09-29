//! The IPC server: a std thread accepting one request per connection, forwarding it to the host
//! thread and writing the reply, or `Failed(Timeout)` after [`REPLY_TIMEOUT`].
//!
//! The accept loop and the per-connection read/dispatch/write are generic over how a request is
//! answered ([`spawn_with`], [`serve_one_with`]) so `crates/sill/tests/ipc.rs` can drive them
//! with a fake dispatcher instead of a running [`HostHandle`] and [`shell_host::Host`], which
//! need a live Wayland display. [`spawn`] and [`serve_one`] are the real dispatcher, over
//! `ShellMsg::Ipc`.

use std::io::BufReader;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use latchkey::{Listening, Stream};
use shell_host::HostHandle;
use sill_ipc::{Failure, FailureKind, IpcError, Request, Response};

use crate::msg::ShellMsg;

/// How long a client waits for the host thread's answer.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// Serve `door` until the host exits (its accept loop then blocks forever on a closed listener;
/// nothing more will connect, and the process exits with the host).
pub fn spawn(door: Listening, host: HostHandle<ShellMsg>) -> JoinHandle<()> {
    spawn_with(door, move |request, reply| {
        dispatch_via_host(&host, request, reply)
    })
}

/// [`spawn`], but answering every request through `dispatch` instead of a running host. `dispatch`
/// is handed the request and a reply channel; it may answer synchronously or hand the channel to
/// another thread, exactly as [`dispatch_via_host`] hands it to [`HostHandle::send`].
pub fn spawn_with(
    door: Listening,
    dispatch: impl Fn(Request, mpsc::SyncSender<Response>) + Send + 'static,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for stream in door.incoming() {
            match stream {
                Ok(stream) => serve_one_with(stream, &dispatch),
                Err(error) => eprintln!("sill: ipc accept: {error}"),
            }
        }
    })
}

/// Read one request from `stream`, hand it to `host`, write the reply.
pub fn serve_one(stream: Stream, host: &HostHandle<ShellMsg>) {
    serve_one_with(stream, &|request, reply| {
        dispatch_via_host(host, request, reply)
    });
}

/// [`serve_one`], but answering through `dispatch` (see [`spawn_with`]).
fn serve_one_with(mut stream: Stream, dispatch: &impl Fn(Request, mpsc::SyncSender<Response>)) {
    let response = match read_request(&mut stream) {
        Ok(request) => answer(dispatch, request),
        Err(error) => malformed(&error),
    };
    let _ = sill_ipc::write_message(&mut stream, &response);
}

fn read_request(stream: &mut Stream) -> Result<Request, IpcError> {
    sill_ipc::read_message(&mut BufReader::new(stream))
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
    use super::{answer, malformed, spawn_with, timeout};
    use sill_ipc::{FailureKind, IpcError, LauncherRequest, MAX_LINE, Request, Response};
    use std::io::BufReader;

    /// A temp-socket `Listening`, so tests never touch the real `sill` agent socket.
    fn temp_door(name: &str) -> (latchkey::Agent, latchkey::Listening) {
        let dir = std::env::temp_dir().join(format!("sill-ipc-test-{name}-{}", std::process::id()));
        let env = latchkey::Environment {
            runtime_dir: Some(dir.as_os_str()),
            ..Default::default()
        };
        let agent = latchkey::Agent::in_environment(name, latchkey::here(), &env)
            .unwrap_or_else(|e| panic!("{e}"));
        let door = agent.listen().unwrap_or_else(|e| panic!("{e}"));
        (agent, door)
    }

    fn round_trip(agent: &latchkey::Agent, request: &Request) -> Response {
        let mut stream = agent
            .connect()
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|| panic!("no daemon listening"));
        sill_ipc::write_message(&mut stream, request).unwrap_or_else(|e| panic!("{e}"));
        sill_ipc::read_message(&mut BufReader::new(stream)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn ping_answers_pong() {
        let (agent, door) = temp_door("ping");
        spawn_with(door, |request, reply| match request {
            Request::Ping => {
                let _ = reply.send(Response::Pong);
            }
            _ => {
                let _ = reply.send(Response::Done);
            }
        });
        assert_eq!(round_trip(&agent, &Request::Ping), Response::Pong);
    }

    #[test]
    fn an_over_long_line_answers_failed_without_reaching_the_dispatcher() {
        let (agent, door) = temp_door("overlong");
        spawn_with(door, |_request, reply| {
            // If this runs, the over-long line was not refused before dispatch.
            let _ = reply.send(Response::Done);
        });
        let long = Request::Launcher(LauncherRequest::Open {
            query: "a".repeat(MAX_LINE),
        });
        let response = round_trip(&agent, &long);
        assert_eq!(
            response,
            Response::Failed(sill_ipc::Failure {
                kind: FailureKind::Internal,
                message: IpcError::TooLong { limit: MAX_LINE }.to_string(),
            })
        );
    }

    #[test]
    fn a_dispatcher_that_never_answers_times_out() {
        let (agent, door) = temp_door("timeout");
        spawn_with(door, |_request, _reply| {
            // Drop the reply sender without sending: a service that never comes back.
        });
        assert_eq!(round_trip(&agent, &Request::Ping), timeout());
    }

    #[test]
    fn answer_falls_back_to_timeout_when_nothing_replies() {
        assert_eq!(answer(&|_req, _reply| {}, Request::Ping), timeout());
    }

    #[test]
    fn malformed_carries_the_parse_error() {
        let error = IpcError::TooLong { limit: 8 };
        assert_eq!(
            malformed(&error),
            Response::Failed(sill_ipc::Failure {
                kind: FailureKind::Internal,
                message: error.to_string(),
            })
        );
    }
}
