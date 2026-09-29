//! A stand-in for logind on a private bus: it owns `org.freedesktop.login1` there and **only
//! records** what sill asks (`Suspend`, `SetIdleHint`, `Inhibit`, `LockSession`), and on command
//! sends the signals sill follows (`PrepareForSleep`, the session's `Lock`) or changes
//! `BlockInhibited`. Nothing here suspends, locks or hints anything. It is how `dev/accept-idle.sh`
//! plays logind.
//!
//! The address is an argument, never `DBUS_SYSTEM_BUS_ADDRESS`, so this cannot reach the real
//! system bus (where logind holds the name) by accident.
//!
//! Commands, one per stdin line: `sleep` (`PrepareForSleep(true)`), `wake`
//! (`PrepareForSleep(false)`), `lock` (the session's `Lock`), `block-idle`, `free-idle`
//! (`BlockInhibited` with and without `idle`). Output, one per stdout line: `ready`, `suspend
//! interactive=<b>`, `idle-hint <b>`, `inhibit <what> <mode>`, `lock-session <id>`, and
//! `sent <command>`.
//!
//! Usage: `cargo run -p sill-testkit --example logind_standin -- <bus address>`

use std::io::Write;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};
use zbus::{fdo, interface};

const NAME: &str = "org.freedesktop.login1";
const MANAGER: &str = "/org/freedesktop/login1";
const SESSION: &str = "/org/freedesktop/login1/session/accept";
const SESSION_ID: &str = "accept";

#[derive(Debug, Clone, Default)]
struct Manager {
    blocked: Arc<Mutex<String>>,
    /// The write ends of handed-out inhibitors, kept so their read ends stay open.
    held: Arc<Mutex<Vec<std::io::PipeWriter>>>,
}

fn session_path() -> fdo::Result<OwnedObjectPath> {
    OwnedObjectPath::try_from(SESSION).map_err(|e| fdo::Error::Failed(e.to_string()))
}

#[interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> fdo::Result<OwnedObjectPath> {
        session_path()
    }

    fn get_session(&self, _id: String) -> fdo::Result<OwnedObjectPath> {
        session_path()
    }

    fn inhibit(
        &self,
        what: String,
        _who: String,
        _why: String,
        mode: String,
    ) -> fdo::Result<OwnedFd> {
        say(&format!("inhibit {what} {mode}"));
        let (reader, writer) = std::io::pipe().map_err(|e| fdo::Error::Failed(e.to_string()))?;
        if let Ok(mut held) = self.held.lock() {
            held.push(writer);
        }
        Ok(OwnedFd::from(std::os::fd::OwnedFd::from(reader)))
    }

    fn suspend(&self, interactive: bool) {
        say(&format!("suspend interactive={interactive}"));
    }

    fn lock_session(&self, id: String) {
        say(&format!("lock-session {id}"));
    }

    #[zbus(property)]
    fn block_inhibited(&self) -> String {
        self.blocked.lock().map(|b| b.clone()).unwrap_or_default()
    }

    #[zbus(property)]
    fn idle_action(&self) -> String {
        "ignore".to_owned()
    }

    #[zbus(signal)]
    async fn prepare_for_sleep(emitter: &SignalEmitter<'_>, start: bool) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Default)]
struct Session;

#[interface(name = "org.freedesktop.login1.Session")]
impl Session {
    fn set_idle_hint(&self, idle: bool) {
        say(&format!("idle-hint {idle}"));
    }

    #[zbus(property)]
    fn id(&self) -> String {
        SESSION_ID.to_owned()
    }

    #[zbus(signal)]
    async fn lock(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: logind_standin <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(serve(address));
}

async fn serve(address: String) {
    let manager = Manager::default();
    let connection = zbus::connection::Builder::address(address.as_str())
        .and_then(|b| b.name(NAME))
        .and_then(|b| b.serve_at(MANAGER, manager.clone()))
        .and_then(|b| b.serve_at(SESSION, Session))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
    let manager_emitter = SignalEmitter::new(&connection, MANAGER)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_owned();
    let session_emitter = SignalEmitter::new(&connection, SESSION)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_owned();
    say("ready");
    let mut lines = stdin_lines();
    while let Some(line) = lines.recv().await {
        let command = line.trim();
        let sent = match command {
            "sleep" => Manager::prepare_for_sleep(&manager_emitter, true).await,
            "wake" => Manager::prepare_for_sleep(&manager_emitter, false).await,
            "lock" => Session::lock(&session_emitter).await,
            "block-idle" | "free-idle" => {
                let text = if command == "block-idle" {
                    "handle-power-key:idle"
                } else {
                    "handle-power-key"
                };
                if let Ok(mut blocked) = manager.blocked.lock() {
                    *blocked = text.to_owned();
                }
                Ok(())
            }
            other => {
                eprintln!("logind_standin: unknown command {other:?}");
                continue;
            }
        };
        match sent {
            Ok(()) => say(&format!("sent {command}")),
            Err(error) => say(&format!("failed {command} {error}")),
        }
    }
}

/// stdin's lines, read on a thread of their own.
fn stdin_lines() -> mpsc::UnboundedReceiver<String> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}
