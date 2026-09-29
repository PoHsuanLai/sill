//! The sill daemon and command line, as a library so `crates/sill/tests/**` can exercise its
//! internals (`main.rs` is a thin wrapper over this crate).

pub mod cli;
pub mod corners;
pub mod daemon;
pub mod edit_widgets;
pub mod idle_dim;
pub mod ipc;
pub mod launch;
pub mod lock;
pub mod notifications;
pub mod osd;
pub mod outputs;
pub mod policy;
pub mod run_args;
pub mod scroll;
