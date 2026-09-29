//! The command-line client: connect to the daemon (starting it if needed), send one request, print
//! the reply.

use std::io::BufReader;
use std::process::ExitCode;
use std::time::Duration;

use sill_ipc::error::IpcError;
use sill_ipc::line::{encode, read_message, write_message};
use sill_ipc::request::Request;
use sill_ipc::response::Response;

/// How long a client waits for a daemon it started.
pub const START_TIMEOUT: Duration = Duration::from_secs(3);

/// Send `request`, print the reply, exit 0 on success.
pub fn main(request: &Request) -> ExitCode {
    match send(request) {
        Ok(Response::Failed(failure)) => {
            eprintln!("sill: {}", failure.message);
            ExitCode::FAILURE
        }
        Ok(response) => match encode(&response) {
            Ok(line) => {
                print!("{line}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("sill: encoding the reply: {error}");
                ExitCode::FAILURE
            }
        },
        Err(ClientError::NotRunning) => {
            eprintln!("sill: no daemon is running");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("sill: {error}");
            ExitCode::FAILURE
        }
    }
}

/// One request's reply. `Quit` and `Ping` never start a daemon.
pub fn send(request: &Request) -> Result<Response, ClientError> {
    let agent = latchkey::Agent::new("sill")?;
    let mut stream = match request {
        Request::Ping | Request::Quit => agent.connect()?.ok_or(ClientError::NotRunning)?,
        _ => agent.connect_or_start(|| latchkey::spawn(&["run"]), START_TIMEOUT)?,
    };
    write_message(&mut stream, request)?;
    let mut reader = BufReader::new(stream);
    Ok(read_message(&mut reader)?)
}

/// Why a request got no reply.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// No daemon, and this request does not start one.
    #[error("no daemon is running")]
    NotRunning,
    /// latchkey could not find or start the daemon.
    #[error("{0}")]
    Agent(#[from] latchkey::Error),
    /// The reply could not be read.
    #[error("{0}")]
    Ipc(#[from] IpcError),
}
