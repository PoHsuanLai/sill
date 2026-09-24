//! The IPC server: a std thread accepting one request per connection, forwarding it to the
//! host thread and writing the reply, or `Failed(Timeout)` after [`REPLY_TIMEOUT`] (plan: "IPC
//! server on a std thread, 2 s reply timeout").

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::thread::JoinHandle;
use std::time::Duration;

use latchkey::{Listening, Stream};
use shell_host::HostHandle;

use crate::msg::ShellMsg;

/// How long a client waits for the host thread's answer.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// Serve `door` until the host exits.
pub fn spawn(door: Listening, host: HostHandle<ShellMsg>) -> JoinHandle<()> {
    todo!("M1 daemon: for each incoming stream, serve_one")
}

/// Read one request from `stream`, hand it to `host`, write the reply.
pub fn serve_one(stream: Stream, host: &HostHandle<ShellMsg>) {
    todo!(
        "M1 daemon: sill_ipc::read_message, ShellMsg::Ipc, recv_timeout(REPLY_TIMEOUT), write_message"
    )
}
