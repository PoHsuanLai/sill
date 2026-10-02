//! The command line: clap's grammar per verb group, and [`request`], the one pure mapping from a
//! parsed command to the IPC request it sends. `run`, `style` and the hidden `auth-check` helper
//! are not requests.

use std::path::PathBuf;

use clap::Parser;

pub mod auth_check;
pub mod client;
mod command;
mod debug;
mod desktop;
mod display;
mod keycap;
mod launcher;
mod made_up;
mod notifications;
pub mod render_gpu;
mod request;
mod session;
pub mod style;
#[cfg(test)]
mod tests;

pub use command::Command;
pub use request::request;

/// sill, the COSMIC shell.
#[derive(Debug, Parser)]
#[command(name = "sill", version, about)]
pub struct Cli {
    /// Write `sill.settings.toml` (the settings schema, design/22 §9.2) into DIR and exit.
    #[arg(long, value_name = "DIR")]
    pub write_schema: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}
