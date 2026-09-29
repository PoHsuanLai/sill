//! A password-free `ext-session-lock-v1` locker for tests, never for a real session:
//!
//! ```sh
//! WAYLAND_DISPLAY=wayland-1 cargo run -p sill-testkit --example session_locker -- <hold-ms>
//! ```
//!
//! Locks the compositor, prints `locked` on stdout once the compositor says so, holds the lock for `hold-ms`, then unlocks and
//! exits 0. `0` is the recovery form: a locker killed without unlocking leaves cosmic-comp
//! locked until another client locks and unlocks (its session-lock handler accepts a new lock
//! once the old client is gone), and this is that client. Exit 3 when the compositor refused the
//! lock (`finished`: someone else holds it), 2 on a usage or connection error.
//!
//! `dev/accept/lock.sh` and `dev/accept/idle.sh` run it on the nested compositor.

use std::process::ExitCode;
use std::time::Duration;

use cosmic_client_toolkit::wayland_client::globals::{GlobalListContents, registry_queue_init};
use cosmic_client_toolkit::wayland_client::protocol::wl_registry;
use cosmic_client_toolkit::wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use cosmic_client_toolkit::wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_v1::{self, ExtSessionLockV1},
};

/// What the compositor has said about our lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Waiting,
    Locked,
    Refused,
}

#[derive(Debug)]
struct State {
    answer: Answer,
}

fn main() -> ExitCode {
    let Some(hold) = std::env::args()
        .nth(1)
        .and_then(|ms| ms.parse::<u64>().ok())
        .map(Duration::from_millis)
    else {
        eprintln!("usage: session_locker <hold-ms>");
        return ExitCode::from(2);
    };
    match run(hold) {
        Ok(Answer::Locked) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(3),
        Err(why) => {
            eprintln!("session_locker: {why}");
            ExitCode::from(2)
        }
    }
}

fn run(hold: Duration) -> Result<Answer, String> {
    let connection = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, mut queue) =
        registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
    let handle = queue.handle();
    let manager: ExtSessionLockManagerV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|e| format!("no ext_session_lock_manager_v1: {e}"))?;
    let lock = manager.lock(&handle, ());
    let mut state = State {
        answer: Answer::Waiting,
    };
    while state.answer == Answer::Waiting {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;
    }
    if state.answer == Answer::Refused {
        lock.destroy();
        let _ = queue.roundtrip(&mut state);
        return Ok(Answer::Refused);
    }
    println!("locked");
    std::thread::sleep(hold);
    lock.unlock_and_destroy();
    finish(&mut queue, &mut state)?;
    println!("unlocked");
    Ok(Answer::Locked)
}

/// One roundtrip so the compositor has processed the unlock before the connection closes.
fn finish(queue: &mut EventQueue<State>, state: &mut State) -> Result<(), String> {
    queue
        .roundtrip(state)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockManagerV1,
        _: <ExtSessionLockManagerV1 as cosmic_client_toolkit::wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.answer = match event {
            ext_session_lock_v1::Event::Locked => Answer::Locked,
            ext_session_lock_v1::Event::Finished => Answer::Refused,
            _ => state.answer,
        };
    }
}
