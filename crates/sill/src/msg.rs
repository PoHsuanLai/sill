//! The daemon's host message: what reaches `Host::run`'s event handler from other threads.

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::sync::mpsc::SyncSender;

use sill_ipc::{Request, Response};
use sill_settings::ShellFile;

/// One message to the host thread (`HostHandle::send`).
#[derive(Debug)]
pub enum ShellMsg {
    /// An IPC request; answer on `reply` within the IPC server's timeout.
    Ipc {
        request: Request,
        reply: SyncSender<Response>,
    },
    /// `settings.toml` settled on a new value.
    Settings(Box<ShellFile>),
    /// Hold or drop shell-host's session lock (M11: `crate::lock::spawn_follower` sends it).
    Lock(crate::lock::LockWish),
    /// Map or unmap the idle dim surfaces (Q420 B: `crate::idle_dim::spawn_follower` sends it).
    Dim(sill_services::idle::DimShown),
}
