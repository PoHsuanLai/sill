//! The sill daemon and command line, as a library so `crates/sill/tests/**` can exercise its
//! internals (`main.rs` is a thin wrapper over this crate).

pub mod cli;
pub mod client;
pub mod daemon;
pub mod ipc_server;
pub mod launch;
pub mod msg;
pub mod outputs;
