//! The sill daemon and command line: `sill run` is the daemon; every other subcommand is a client
//! that sends one request and prints the reply.
//!
//! The implementation lives in the `sill` library (`lib.rs`) so `crates/sill/tests/**` can
//! exercise it; this binary is a thin wrapper.

use std::process::ExitCode;

use clap::Parser;
use sill::cli;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    if let Some(dir) = &cli.write_schema {
        return match sill_settings::schema().write_to(dir) {
            Ok(path) => sill::cli::stdout::print(&format!("{}\n", path.display())).exit_code(),
            Err(error) => {
                eprintln!("sill: writing the schema to {}: {error}", dir.display());
                ExitCode::FAILURE
            }
        };
    }
    match cli.command {
        Some(cli::Command::Run(args)) => sill::daemon::run(&args),
        Some(cli::Command::AuthCheck) => sill::cli::auth_check::main(),
        Some(cli::Command::Style { action }) => sill::cli::style::main(&action),
        Some(command) => match cli::request(&command) {
            Some(request) => sill::cli::client::main(&request),
            None => ExitCode::SUCCESS,
        },
        None => {
            eprintln!("sill: a subcommand is required (sill --help)");
            ExitCode::from(2)
        }
    }
}
