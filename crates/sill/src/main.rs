//! The sill daemon and command line (plan "CLI/IPC/session"): `sill run` is the daemon; every
//! other subcommand is a client that sends one request and prints the reply.
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
            Ok(path) => {
                println!("{}", path.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("sill: writing the schema to {}: {error}", dir.display());
                ExitCode::FAILURE
            }
        };
    }
    match cli.command {
        Some(cli::Command::Run) => sill::daemon::run(),
        Some(cli::Command::PolkitAgent { verb }) => sill::session::polkit_agent(verb),
        Some(cli::Command::AuthCheck) => sill::auth_check::main(),
        Some(command) => match cli::request(&command) {
            Some(request) => sill::client::main(&request),
            None => ExitCode::SUCCESS,
        },
        None => {
            eprintln!("sill: a subcommand is required (sill --help)");
            ExitCode::from(2)
        }
    }
}
