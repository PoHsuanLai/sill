//! The IPC server's public surface, against a fake dispatcher on a temp socket: no running host, no
//! Wayland display, no `Services`. Checks Ping -> Pong, an over-long line -> `Failed`, and the 2 s
//! reply timeout, and that a client that says nothing holds up nobody.

use std::io::BufReader;
use std::time::{Duration, Instant};

use sill::ipc::server;
use sill_ipc::line::{MAX_LINE, read_message, write_message};
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_ipc::show::LauncherRequest;

/// A `Listening` on a throwaway socket under a per-test temp directory, so this suite never
/// touches the real `sill` agent's socket even when it runs alongside a live daemon.
fn temp_door(name: &str) -> (latchkey::Agent, latchkey::Listening) {
    // Short on purpose: the socket path must fit sun_path (108 bytes) under any TMPDIR; the pid
    // and the clock already make it unique, and `name` is only for messages.
    let dir = std::env::temp_dir().join(format!(
        "sill-ipc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let env = latchkey::Environment {
        runtime_dir: Some(dir.as_os_str().to_owned()),
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
    let request = Request::Launcher(LauncherRequest::Open {
        query: "a".repeat(MAX_LINE),
    });
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

#[test]
fn a_silent_client_does_not_hold_up_the_next_one() {
    let (agent, door) = temp_door("silent");
    server::spawn_with(door, |_request, reply| {
        let _ = reply.send(Response::Pong);
    });
    // Connects and sends nothing, and stays connected for the whole test.
    let _silent = agent
        .connect()
        .unwrap_or_else(|e| panic!("connecting: {e}"))
        .unwrap_or_else(|| panic!("nobody is listening"));
    let started = Instant::now();
    assert_eq!(round_trip(&agent, &Request::Ping), Response::Pong);
    assert!(
        started.elapsed() < server::READ_DEADLINE,
        "the second client waited {:?} behind the silent one",
        started.elapsed()
    );
}

#[test]
fn a_client_that_never_finishes_its_line_is_cut_at_the_deadline() {
    use std::io::Read;
    let (agent, door) = temp_door("stalled");
    server::spawn_with(door, |_request, reply| {
        let _ = reply.send(Response::Done);
    });
    let mut stalled = agent
        .connect()
        .unwrap_or_else(|e| panic!("connecting: {e}"))
        .unwrap_or_else(|| panic!("nobody is listening"));
    std::io::Write::write_all(&mut stalled, b"{\"kind\":").unwrap_or_else(|e| panic!("{e}"));
    let started = Instant::now();
    let mut reply = String::new();
    // The server answers the half line with a failure and closes: the read ends.
    let _ = stalled.read_to_string(&mut reply);
    assert!(
        started.elapsed() < server::READ_DEADLINE + Duration::from_secs(2),
        "the half line was still held after {:?}",
        started.elapsed()
    );
    assert!(
        reply.contains("failed"),
        "a failure is the answer: {reply:?}"
    );
}

#[test]
fn a_stopped_server_ends_its_accept_loop_and_serves_no_more() {
    let (agent, door) = temp_door("stopped");
    let serving = server::spawn_with(door, |_request, reply| {
        let _ = reply.send(Response::Pong);
    });
    assert_eq!(round_trip(&agent, &Request::Ping), Response::Pong);
    let thread = serving.stop(&agent);
    let waited = Instant::now();
    while !thread.is_finished() {
        assert!(
            waited.elapsed() < Duration::from_secs(5),
            "the accept loop was still blocked 5 s after it was stopped"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    thread
        .join()
        .unwrap_or_else(|_| panic!("the accept thread panicked"));
}

/// `sill ping` against a real, already-running daemon. Not run by `cargo test --workspace`
/// (no daemon exists in that environment); a developer with one running invokes it with
/// `cargo test -p sill --test it ipc -- --ignored`.
#[test]
#[ignore = "needs a running `sill run` daemon"]
fn ping_reaches_a_running_daemon() {
    let response = sill::cli::client::send(&Request::Ping).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(response, Response::Pong);
}
