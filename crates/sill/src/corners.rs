//! Hot corners on the daemon's side: the actions a firing carries out. The corner surfaces are
//! spawned like any `Surface` (`sill_overlays::hot_corners::HotCorners`).
//!
//! A firing arrives on the [`CornerLink`] (from a corner surface's machine, or from
//! `sill debug corner`); [`spawn_hits`] follows it and turns that corner's setting into
//! [`corner_effect`]: an action of the shell, handed to the interpreter (`crate::actions`) with the
//! corner's output as its origin, or a custom command run as a child process.

use std::process::Stdio;

use shell_host::{HostHandle, OutputId};
use sill_launcher::actions::ActionId;
use sill_model::actions::Ask;
use sill_settings::{CornerAction, ShellFile};
use sill_ui_kit::links::{Asked, Links, Origin};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

/// What a corner's action does, resolved from the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CornerEffect {
    Nothing,
    /// An action of the shell, on the corner's output.
    Do(ActionId),
    /// A child process: `/bin/sh -c COMMAND` for `Command`.
    Run {
        program: String,
        args: Vec<String>,
    },
}

/// What `action` does; `command` is the corner's `*_command` (a `Command` with an empty one does
/// nothing).
pub fn corner_effect(action: CornerAction, command: &str) -> CornerEffect {
    match action {
        CornerAction::None => CornerEffect::Nothing,
        CornerAction::Launcher => CornerEffect::Do(ActionId::Launcher),
        CornerAction::NotificationCenter => CornerEffect::Do(ActionId::NotificationCenter),
        CornerAction::ControlCenter => CornerEffect::Do(ActionId::ControlCenter),
        CornerAction::ShowDesktop => CornerEffect::Do(ActionId::ShowDesktop),
        CornerAction::Lock => CornerEffect::Do(ActionId::Lock),
        CornerAction::Workspaces => CornerEffect::Do(ActionId::MissionControl),
        CornerAction::Command if command.trim().is_empty() => CornerEffect::Nothing,
        CornerAction::Command => CornerEffect::Run {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), command.to_owned()],
        },
        // `CornerAction` is a foreign `#[non_exhaustive]` enum, so the compiler asks for this arm
        // and no other: a variant added upstream lands here until it is handled above, and it
        // must be seen rather than silently do nothing. The test below lists every variant.
        other => {
            eprintln!(
                "sill: a hot corner action this build does not know ({other:?}) does nothing"
            );
            CornerEffect::Nothing
        }
    }
}

/// Follow the corner link on `rt` and carry out each hit's action (read from `settings` at the
/// hit): an action goes to the host thread's interpreter, a command is run here.
pub fn spawn_hits(
    rt: &Handle,
    links: Links,
    settings: watch::Receiver<ShellFile>,
    host: HostHandle<ShellMsg>,
) {
    let mut hits = links.corners.subscribe();
    rt.spawn(async move {
        loop {
            if hits.changed().await.is_err() {
                return;
            }
            let Some(hit) = *hits.borrow_and_update() else {
                continue;
            };
            let file = settings.borrow().clone();
            let action = sill_overlays::hot_corners::corner_action(&file.hot_corners, hit.corner);
            let command = sill_overlays::hot_corners::corner_command(&file.hot_corners, hit.corner)
                .to_owned();
            act(corner_effect(action, &command), hit.output, &host);
        }
    });
}

/// Carry out `effect` on `output`: hand an action to the host with the corner's output as its
/// origin; spawn a detached child process for `Run` (a custom command).
fn act(effect: CornerEffect, output: OutputId, host: &HostHandle<ShellMsg>) {
    match effect {
        CornerEffect::Nothing => {}
        CornerEffect::Do(action) => {
            let origin = Origin {
                surface: None,
                output: Some(output),
            };
            let asked = Asked {
                ask: Ask::of(action),
                origin,
            };
            // The host is gone: the daemon is ending.
            let _gone_is_dropped = host.send(ShellMsg::Action(asked));
        }
        CornerEffect::Run { program, args } => {
            let mut command = tokio::process::Command::new(&program);
            command.args(&args).stdin(Stdio::null()).kill_on_drop(false);
            if let Err(error) = command.spawn() {
                eprintln!("sill: hot corner: running {program}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sill_launcher::actions::ActionId;
    use sill_settings::CornerAction;

    use super::{CornerEffect, corner_effect};

    #[test]
    fn each_action_has_its_effect() {
        let run = |program: &str, args: &[&str]| CornerEffect::Run {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        };
        let cases = [
            (CornerAction::None, "", CornerEffect::Nothing),
            (
                CornerAction::Launcher,
                "",
                CornerEffect::Do(ActionId::Launcher),
            ),
            (
                CornerAction::NotificationCenter,
                "",
                CornerEffect::Do(ActionId::NotificationCenter),
            ),
            (
                CornerAction::ControlCenter,
                "",
                CornerEffect::Do(ActionId::ControlCenter),
            ),
            (
                CornerAction::ShowDesktop,
                "",
                CornerEffect::Do(ActionId::ShowDesktop),
            ),
            (CornerAction::Lock, "", CornerEffect::Do(ActionId::Lock)),
            (
                CornerAction::Workspaces,
                "",
                CornerEffect::Do(ActionId::MissionControl),
            ),
            (
                CornerAction::Command,
                "foot -e htop",
                run("/bin/sh", &["-c", "foot -e htop"]),
            ),
            (CornerAction::Command, "  ", CornerEffect::Nothing),
            (
                CornerAction::Launcher,
                "ignored",
                CornerEffect::Do(ActionId::Launcher),
            ),
        ];
        for (action, command, want) in cases {
            assert_eq!(
                corner_effect(action, command),
                want,
                "{action:?} {command:?}"
            );
        }
    }
}
