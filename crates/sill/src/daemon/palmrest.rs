//! What palmrest asks of the session. palmrest is a system service with no session of its own, so
//! a gesture whose action needs one (the workspace steps, a line of the shell's own commands, a
//! program of the person's) arrives here as `HostEvent::Palmrest` and the shell carries it out as
//! the person: the workspace steps and shell lines are requests, answered as `sill workspace next`
//! and the like are, and a program is started as a child of the shell. The decision is pure
//! ([`plan`]); only [`run`] has an effect.

use std::iter::once;
use std::process::Stdio;

use clap::Parser;
use shell_host::{PalmrestAction, PalmrestWorkspace};
use sill_ipc::request::Request;

use crate::cli::{Cli, request};

/// What an ask comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// A request, carried out as a client's would be.
    Request(Request),
    /// A program of the person's, started without a shell.
    Program { program: String, args: Vec<String> },
    /// Not something the shell does for palmrest.
    Refused(&'static str),
}

/// What `ask` comes to.
pub fn plan(ask: &PalmrestAction) -> Plan {
    match ask {
        PalmrestAction::Workspace(PalmrestWorkspace::Prev) => line(&["workspace", "prev"]),
        PalmrestAction::Workspace(PalmrestWorkspace::Next) => line(&["workspace", "next"]),
        PalmrestAction::Workspace(PalmrestWorkspace::Overview) => line(&["workspace", "overview"]),
        PalmrestAction::Shell { words } => line(words),
        PalmrestAction::Command { argv } => match argv.split_first() {
            Some((program, args)) => Plan::Program {
                program: program.clone(),
                args: args.to_vec(),
            },
            None => Plan::Refused("a program with no name"),
        },
    }
}

/// The request a line of the shell's command language sends. Stopping the shell and the
/// development probes are not something a gesture does.
fn line<S: AsRef<str>>(words: &[S]) -> Plan {
    let argv = once("sill").chain(words.iter().map(AsRef::as_ref));
    let request = Cli::try_parse_from(argv)
        .ok()
        .and_then(|cli| cli.command)
        .and_then(|command| request(&command));
    match request {
        Some(Request::Quit | Request::Debug(_)) => Plan::Refused("a gesture does not do that"),
        Some(request) => Plan::Request(request),
        None => Plan::Refused("not a command of the shell"),
    }
}

/// Starts `program` with `args` as a child of the shell, and waits for it on a thread of its own,
/// so a finished program is not left a zombie.
pub fn run(program: &str, args: &[String]) {
    match std::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let reaped = std::thread::Builder::new()
                .name("sill-palmrest-reaper".to_owned())
                .spawn(move || {
                    let _ = child.wait();
                });
            if let Err(error) = reaped {
                eprintln!("sill: palmrest: could not wait for {program}: {error}");
            }
        }
        Err(error) => eprintln!("sill: palmrest: running {program}: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sill_ipc::act::{ActRequest, WorkspaceRequest};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn each_ask_comes_to_what_the_command_line_would_send() {
        let step = |w| Plan::Request(Request::Act(ActRequest::Workspace(w)));
        let cases = [
            (
                "previous workspace",
                PalmrestAction::Workspace(PalmrestWorkspace::Prev),
                step(WorkspaceRequest::Prev),
            ),
            (
                "next workspace",
                PalmrestAction::Workspace(PalmrestWorkspace::Next),
                step(WorkspaceRequest::Next),
            ),
            (
                "the overview",
                PalmrestAction::Workspace(PalmrestWorkspace::Overview),
                step(WorkspaceRequest::Overview),
            ),
            (
                "a line of the shell's commands",
                PalmrestAction::Shell {
                    words: words("workspace next"),
                },
                step(WorkspaceRequest::Next),
            ),
            (
                "a program",
                PalmrestAction::Command {
                    argv: words("notify-send hi"),
                },
                Plan::Program {
                    program: "notify-send".into(),
                    args: vec!["hi".into()],
                },
            ),
        ];
        for (name, ask, want) in cases {
            assert_eq!(plan(&ask), want, "{name}");
        }
    }

    #[test]
    fn what_a_gesture_may_not_do_or_is_not_a_command_is_refused() {
        let cases = [
            (
                "stopping the shell",
                PalmrestAction::Shell {
                    words: words("quit"),
                },
            ),
            (
                "not a command",
                PalmrestAction::Shell {
                    words: words("no such verb"),
                },
            ),
            (
                "the shell's own start",
                PalmrestAction::Shell {
                    words: words("run"),
                },
            ),
            (
                "a help request",
                PalmrestAction::Shell {
                    words: words("--help"),
                },
            ),
            ("no program", PalmrestAction::Command { argv: vec![] }),
        ];
        for (name, ask) in cases {
            assert!(
                matches!(plan(&ask), Plan::Refused(_)),
                "{name}: {:?}",
                plan(&ask)
            );
        }
    }
}
