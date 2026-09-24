//! The command-line client: connect to the daemon (starting it if needed), send one request,
//! print the reply (plan: "clients `connect_or_start(|| spawn([\"run\"]), 3 s)`").

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::process::ExitCode;
use std::time::Duration;

use sill_ipc::{Request, Response};

/// How long a client waits for a daemon it started.
pub const START_TIMEOUT: Duration = Duration::from_secs(3);

/// Send `request`, print the reply, exit 0 on success.
pub fn main(request: &Request) -> ExitCode {
    todo!("M1 daemon: send, then print Pong/Done/Stats as JSON lines, Failed to stderr")
}

/// One request's reply. `Quit` and `Ping` never start a daemon.
pub fn send(request: &Request) -> Result<Response, ClientError> {
    todo!("M1 daemon: latchkey::Agent::new(\"sill\"), connect or connect_or_start")
}

/// Why a request got no reply.
#[derive(Debug)]
pub enum ClientError {
    /// No daemon, and this request does not start one.
    NotRunning,
    /// latchkey could not find or start the daemon.
    Agent(latchkey::Error),
    /// The reply could not be read.
    Ipc(sill_ipc::IpcError),
}
