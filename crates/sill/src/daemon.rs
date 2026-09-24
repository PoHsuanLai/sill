//! `sill run`: take the single-instance lock, start the services and the IPC server, connect
//! the host, and keep one set of surfaces per output (plan: "daemon uses
//! `latchkey::Agent::new(\"sill\").listen()` (AlreadyRunning -> exit 0)").

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::process::ExitCode;

use shell_host::{HostCtx, HostEvent};

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Run the daemon until `quit` or a fatal host error.
pub fn run() -> ExitCode {
    todo!(
        "M1 daemon: latchkey listen, tokio runtime, Services::spawn, settings watch, ipc_server::spawn, Host::connect, Host::run(on_event)"
    )
}

/// The host event handler: outputs come and go, IPC requests are answered, surfaces closed by
/// the compositor are respawned.
pub fn on_event(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs, event: HostEvent<ShellMsg>) {
    todo!("M1 daemon")
}
