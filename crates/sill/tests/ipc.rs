//! The IPC server's public surface, against a fake dispatcher on a temp socket: no running host, no
//! Wayland display, no `Services`. Checks Ping -> Pong, an over-long line -> `Failed`, and the 2 s
//! reply timeout.

use std::io::BufReader;
use std::time::{Duration, Instant};

use sill::ipc::server;
use sill_ipc::line::{MAX_LINE, read_message, write_message};
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_ipc::show::{LauncherRequest, ShowRequest};

/// A `Listening` on a throwaway socket under a per-test temp directory, so this suite never
/// touches the real `sill` agent's socket even when it runs alongside a live daemon.
fn temp_door(name: &str) -> (latchkey::Agent, latchkey::Listening) {
    let dir = std::env::temp_dir().join(format!(
        "sill-ipc-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let env = latchkey::Environment {
        runtime_dir: Some(dir.as_os_str()),
        ..Default::default()
    };
    let agent = latchkey::Agent::in_environment(name, latchkey::here(), &env)
        .unwrap_or_else(|e| panic!("{name}: building the agent: {e}"));
    let door = agent
        .listen()
        .unwrap_or_else(|e| panic!("{name}: listening: {e}"));
    (agent, door)
}

fn round_trip(agent: &latchkey::Agent, request: &Request) -> Response {
    let mut stream = agent
        .connect()
        .unwrap_or_else(|e| panic!("connecting: {e}"))
        .unwrap_or_else(|| panic!("nobody is listening"));
    write_message(&mut stream, request).unwrap_or_else(|e| panic!("writing: {e}"));
    read_message(&mut BufReader::new(stream)).unwrap_or_else(|e| panic!("reading: {e}"))
}

#[test]
fn ping_answers_pong() {
    let (agent, door) = temp_door("ping");
    server::spawn_with(door, |request, reply| {
        let response = match request {
            Request::Ping => Response::Pong,
            _ => Response::Done,
        };
        let _ = reply.send(response);
    });
    assert_eq!(round_trip(&agent, &Request::Ping), Response::Pong);
}

#[test]
fn an_over_long_line_answers_failed() {
    let (agent, door) = temp_door("overlong");
    server::spawn_with(door, |_request, reply| {
        // If the dispatcher is reached at all, the over-long line was not refused first.
        let _ = reply.send(Response::Done);
    });
    let request = Request::Show(ShowRequest::Launcher(LauncherRequest::Open {
        query: "a".repeat(MAX_LINE),
    }));
    let response = round_trip(&agent, &request);
    let Response::Failed(failure) = response else {
        panic!("expected Failed, got {response:?}");
    };
    assert_eq!(failure.kind, FailureKind::Internal);
}

#[test]
fn a_silent_dispatcher_times_out_within_the_budget() {
    let (agent, door) = temp_door("timeout");
    server::spawn_with(door, |_request, _reply| {
        // Drop the reply sender without sending: a service that never comes back.
    });
    let started = Instant::now();
    let response = round_trip(&agent, &Request::Ping);
    let elapsed = started.elapsed();
    let Response::Failed(failure) = response else {
        panic!("expected Failed, got {response:?}");
    };
    assert_eq!(failure.kind, FailureKind::Timeout);
    assert!(
        elapsed < Duration::from_secs(3),
        "took {elapsed:?}, expected close to the 2 s reply timeout"
    );
}

/// `sill ping` against a real, already-running daemon. Not run by `cargo test --workspace`
/// (no daemon exists in that environment); a developer with one running invokes it with
/// `cargo test -p sill --test ipc -- --ignored`.
#[test]
#[ignore = "needs a running `sill run` daemon"]
fn ping_reaches_a_running_daemon() {
    let response = sill::cli::client::send(&Request::Ping).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(response, Response::Pong);
}
