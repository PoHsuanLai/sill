//! The command line: clap's grammar per verb group, and [`request`], the one pure mapping from a
//! parsed command to the IPC request it sends. `run`, `style` and the hidden `auth-check` helper
//! are not requests.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

pub mod auth_check;
pub mod client;
mod command;
mod debug;
mod desktop;
mod display;
mod intents;
mod keycap;
pub mod launch;
mod launcher;
mod made_up;
mod notifications;
mod request;
mod session;
pub mod stdout;
pub mod style;
#[cfg(test)]
mod tests;

pub use command::Command;
pub use request::request;

/// Parse the process's arguments. Help and version print and end with 0; any other mistake prints
/// clap's message and ends with 64 (`EX_USAGE`), which leaves 2 for `sill launch`'s unknown id.
pub fn parse_args() -> Result<Cli, ExitCode> {
    Cli::try_parse().map_err(|error| {
        // A message that cannot be printed leaves nothing more to say.
        let _unprintable_is_dropped = error.print();
        ExitCode::from(usage_code(&error))
    })
}

/// The exit code for a command line clap refused: 0 for help and version, else 64.
fn usage_code(error: &clap::Error) -> u8 {
    if error.use_stderr() { USAGE } else { 0 }
}

/// `EX_USAGE` of sysexits.h: the command line was wrong.
pub const USAGE: u8 = 64;

/// sill, the desktop shell.
#[derive(Debug, Parser)]
#[command(name = "sill", version, about)]
pub struct Cli {
    /// Write `sill.settings.toml` (the settings schema, design/22 §9.2) into DIR and exit.
    #[arg(long, value_name = "DIR")]
    pub write_schema: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}
