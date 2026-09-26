//! The sill daemon and command line, as a library so `crates/sill/tests/**` can exercise its
//! internals (`main.rs` is a thin wrapper over this crate).

pub mod auth_check;
pub mod calendar;
pub mod cli;
pub mod cli_desktop;
pub mod cli_display;
pub mod cli_launcher;
pub mod cli_notifications;
pub mod cli_session;
pub mod client;
pub mod clipboard;
pub mod corners;
pub mod daemon;
pub mod displays;
pub mod ipc_server;
pub mod launch;
pub mod launcher_debug;
pub mod launcher_policy;
pub mod links;
pub mod lock;
pub mod msg;
pub mod notifications;
pub mod osd;
pub mod outputs;
pub mod polkit;
pub mod power_menu;
pub mod screenshots;
pub mod scroll;
pub mod session;
pub mod switcher;
pub mod widgets;
