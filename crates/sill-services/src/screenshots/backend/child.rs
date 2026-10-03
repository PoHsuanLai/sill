//! The service's children: the opener and the trash (`xdg-open`, `gio trash`), started through
//! `Env::children` (a test records them), and the tool, whose exit the service waits for. The tool
//! is found on `PATH`, so a test puts a stand-in first.

use std::ffi::OsStr;
use std::process::Stdio;

use sill_model::screenshots::CommandLine;
use tokio::process::Command;

use crate::env::{Children, ProcessGroup, Start};

/// Start `program` with `args` through `children`; a failure to start is logged.
pub(crate) fn detach(children: &dyn Children, program: &str, args: &[&OsStr]) {
    let start = Start {
        argv: std::iter::once(OsStr::new(program))
            .chain(args.iter().copied())
            .map(OsStr::to_owned)
            .collect(),
        token: None,
        group: ProcessGroup::Shared,
    };
    if let Err(error) = children.detach(&start) {
        eprintln!("screenshots: {program}: {error}");
    }
}

/// Run `line` and call `ended` once it exits (or could not start).
pub(crate) fn run_then(line: CommandLine, ended: impl FnOnce() + Send + 'static) {
    let mut command = Command::new(&line.program);
    command.args(&line.args);
    wait_then(command, &line.program, ended);
}

fn wait_then(mut command: Command, program: &str, ended: impl FnOnce() + Send + 'static) {
    command.stdin(Stdio::null());
    match command.spawn() {
        Ok(mut child) => {
            tokio::spawn(async move {
                let _ = child.wait().await;
                ended();
            });
        }
        Err(error) => {
            eprintln!("screenshots: {program}: {error}");
            ended();
        }
    }
}
