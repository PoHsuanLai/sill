//! The service's children: the opener and the trash (`xdg-open`, `gio trash`), started and
//! reaped, and the tool, whose exit the service waits for. Found on `PATH`, so a test puts a
//! stand-in first.

use std::ffi::OsStr;
use std::process::Stdio;

use tokio::process::Command;

use super::tool::CommandLine;

/// Start `program` with `args` and reap it in the background; a failure to start is logged.
pub(super) fn detach(program: &str, args: &[&OsStr]) {
    let mut command = Command::new(program);
    command.args(args);
    wait_then(command, program, || {});
}

/// Run `line` and call `ended` once it exits (or could not start).
pub(super) fn run_then(line: CommandLine, ended: impl FnOnce() + Send + 'static) {
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
