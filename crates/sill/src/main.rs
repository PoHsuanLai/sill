//! The sill daemon and command line (plan "CLI/IPC/session"): `sill run` is the daemon; every
//! other subcommand is a client that sends one request and prints the reply.

mod cli;
mod client;
mod daemon;
mod ipc_server;
mod launch;
mod msg;
mod outputs;

use std::process::ExitCode;

use clap::Parser;

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
        Some(cli::Command::Run) => daemon::run(),
        Some(command) => match cli::request(&command) {
            Some(request) => client::main(&request),
            None => ExitCode::SUCCESS,
        },
        None => {
            eprintln!("sill: a subcommand is required (sill --help)");
            ExitCode::from(2)
        }
    }
}
