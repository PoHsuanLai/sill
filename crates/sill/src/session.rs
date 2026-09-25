//! M7's session parts on the daemon's side (FINDINGS "M7"): the locker's policy from
//! `session.locker`, `sill lock`, and `sill polkit-agent`, which resolves `session.polkit_agent`
//! and execs the agent so `dist/sill-session` can supervise it like any other component.

use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

use sill_ipc::{Failure, FailureKind, Response};
use sill_services::polkit::{AgentChoice, AgentMode, resolve};
use sill_services::{Control, LockerChoice, LockerCommand, Services};
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::cli::AgentVerb;

/// Sends `session.locker` to the locker service now and whenever it changes.
pub fn spawn_locker_policy(
    rt: &Handle,
    mut settings: watch::Receiver<ShellFile>,
    control: Control<LockerCommand>,
) {
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let choice = LockerChoice::parse(&settings.borrow_and_update().session.locker);
            if sent.as_ref() != Some(&choice) {
                if control
                    .send(LockerCommand::Configure(choice.clone()))
                    .is_err()
                {
                    return;
                }
                sent = Some(choice);
            }
            if settings.changed().await.is_err() {
                return;
            }
        }
    });
}

/// `sill lock`.
pub fn lock(services: &Services) -> Response {
    match services.locker_control.send(LockerCommand::Lock) {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: format!("the locker service: {error}"),
        }),
    }
}

/// `sill polkit-agent [exec|print]`: resolve `session.polkit_agent` from the settings file and
/// replace this process with the agent (or print it). Exits 0 with nothing to run when the
/// setting is `off` or no agent is installed, so a supervisor does not spin on it.
pub fn polkit_agent(verb: AgentVerb) -> ExitCode {
    let file = sill_settings::settings_dir()
        .as_deref()
        .map(sill_settings::load)
        .unwrap_or_default();
    let choice = AgentChoice::parse(&file.session.polkit_agent);
    let mode = resolve(&choice, sill_services::programs::find);
    match (verb, mode) {
        (AgentVerb::Print, mode) => {
            if let Some(line) = command_line(&mode) {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        (AgentVerb::Exec, AgentMode::Off) => {
            eprintln!("sill: polkit-agent: none configured or installed ({choice:?})");
            ExitCode::SUCCESS
        }
        (AgentVerb::Exec, AgentMode::Run { program, args }) => {
            let error = Command::new(&program).args(&args).exec();
            eprintln!(
                "sill: polkit-agent: starting {}: {error}",
                program.display()
            );
            ExitCode::FAILURE
        }
    }
}

/// `mode` as one line for `print`; `None` when nothing runs.
pub fn command_line(mode: &AgentMode) -> Option<String> {
    match mode {
        AgentMode::Off => None,
        AgentMode::Run { program, args } => Some(
            std::iter::once(program.display().to_string())
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(" "),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::command_line;
    use sill_services::polkit::AgentMode;

    #[test]
    fn printed_lines() {
        let cases = [
            (AgentMode::Off, None),
            (
                AgentMode::Run {
                    program: PathBuf::from("/usr/libexec/kf6/polkit-kde-authentication-agent-1"),
                    args: Vec::new(),
                },
                Some("/usr/libexec/kf6/polkit-kde-authentication-agent-1"),
            ),
            (
                AgentMode::Run {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".to_owned(), "my-agent".to_owned()],
                },
                Some("/bin/sh -c my-agent"),
            ),
        ];
        for (mode, want) in cases {
            assert_eq!(command_line(&mode).as_deref(), want, "{mode:?}");
        }
    }
}
