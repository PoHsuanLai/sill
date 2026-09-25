//! The sill daemon and command line, as a library so `crates/sill/tests/**` can exercise its
//! internals (`main.rs` is a thin wrapper over this crate).

pub mod cli;
pub mod cli_display;
pub mod client;
pub mod daemon;
pub mod displays;
pub mod ipc_server;
pub mod launch;
pub mod links;
pub mod msg;
pub mod osd;
pub mod outputs;
pub mod power_menu;
