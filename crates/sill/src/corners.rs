//! Hot corners on the daemon's side: the actions a firing carries out. The corner surfaces are
//! spawned like any `Surface` (`sill_overlays::hot_corners::HotCorners`).
//!
//! A firing arrives on the [`CornerLink`] (from a corner surface's machine, or from
//! `sill debug corner`); [`spawn_hits`] follows it and carries out [`corner_effect`] of that corner's
//! action: the launcher, notification center and control center through their links, Lock
//! through the locker service, Show Desktop through the toplevel
//! manager ([`show_desktop`]), the workspace overview (Mission Control) as a command for the
//! compositor's arm, and a custom command as a child process.

use std::process::Stdio;

use shell_host::{HostHandle, OutputId};
use sill_launcher::WindowKey;
use sill_model::desktop::open_mission_control;
use sill_model::ids::WorkspaceKey;
use sill_model::locker::LockerCommand;
use sill_model::windows::{Activity, Toplevel, WindowState, WlCommand};
use sill_settings::{CornerAction, ShellFile};
use sill_ui_kit::links::Links;
use sill_ui_kit::services::Services;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

/// What a corner's action does, resolved from the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CornerEffect {
    Nothing,
    Launcher,
    NotificationCenter,
    ControlCenter,
    ShowDesktop,
    Lock,
    /// Mission Control, on the desktop's own compositor.
    MissionControl,
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
        CornerAction::Launcher => CornerEffect::Launcher,
        CornerAction::NotificationCenter => CornerEffect::NotificationCenter,
        CornerAction::ControlCenter => CornerEffect::ControlCenter,
        CornerAction::ShowDesktop => CornerEffect::ShowDesktop,
        CornerAction::Lock => CornerEffect::Lock,
        CornerAction::Workspaces => CornerEffect::MissionControl,
        CornerAction::Command if command.trim().is_empty() => CornerEffect::Nothing,
        CornerAction::Command => CornerEffect::Run {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), command.to_owned()],
        },
        _ => CornerEffect::Nothing,
    }
}

/// The windows Show Desktop minimised, to bring back on its next firing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stash(pub Vec<WindowKey>);

/// Show Desktop, as macOS's: when `stash` holds a window still minimised, unminimise the
/// stash's minimised windows (in their order) and empty it; otherwise minimise every normal
/// window on one of the `active` workspaces and stash them.
pub fn show_desktop(
    windows: &[Toplevel],
    active: &[WorkspaceKey],
    stash: &Stash,
) -> (Vec<WlCommand>, Stash) {
    let restore: Vec<WindowKey> = stash
        .0
        .iter()
        .copied()
        .filter(|key| {
            windows
                .iter()
                .any(|w| w.key == *key && w.window == WindowState::Minimized)
        })
        .collect();
    if !restore.is_empty() {
        let commands = restore.into_iter().map(WlCommand::Unminimize).collect();
        return (commands, Stash::default());
    }
    let minimize: Vec<WindowKey> = windows
        .iter()
        .filter(|w| {
            w.window == WindowState::Normal && w.workspaces.iter().any(|ws| active.contains(ws))
        })
        .map(|w| w.key)
        .collect();
    let commands = minimize.iter().copied().map(WlCommand::Minimize).collect();
    (commands, Stash(minimize))
}

/// Follow `link` on `rt` and carry out each hit's action (read from `settings` at the hit)
/// through `services` and `links`.
pub fn spawn_hits(
    rt: &Handle,
    links: Links,
    settings: watch::Receiver<ShellFile>,
    services: Services,
    host: HostHandle<ShellMsg>,
) {
    let mut hits = links.corners.subscribe();
    rt.spawn(async move {
        let mut stash = Stash::default();
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
            let effect = corner_effect(action, &command);
            act(effect, hit.output, &services, &links, &host, &mut stash).await;
        }
    });
}

/// Carry out `effect` on `output`: toggle the launcher, notification center or control center;
/// minimise (or restore) windows for Show Desktop; lock through the session's locker; open
/// Mission Control through the host; spawn a detached child process for `Run` (a custom command).
async fn act(
    effect: CornerEffect,
    output: OutputId,
    services: &Services,
    links: &Links,
    host: &HostHandle<ShellMsg>,
    stash: &mut Stash,
) {
    match effect {
        CornerEffect::Nothing => {}
        CornerEffect::Launcher => links.launcher.toggle(output),
        CornerEffect::NotificationCenter => {
            links.notification_center.toggle(output);
        }
        CornerEffect::ControlCenter => {
            links.control_center.toggle(output);
        }
        CornerEffect::ShowDesktop => {
            let wl = services.windows.state.borrow().clone();
            let windows = wl.toplevels.windows;
            let active: Vec<WorkspaceKey> = wl
                .workspaces
                .groups
                .iter()
                .filter_map(|group| {
                    group
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.activity == Activity::Active)
                        .map(|workspace| workspace.key)
                })
                .collect();
            let (commands, next) = show_desktop(&windows, &active, stash);
            *stash = next;
            for command in commands {
                let _ = services.windows.control.send(command);
            }
        }
        CornerEffect::Lock => {
            let _ = services.locker.control.send(LockerCommand::Lock);
        }
        CornerEffect::MissionControl => {
            // Without the desktop's own compositor the host has no arm and drops the command.
            let _ = host.send(ShellMsg::Command(open_mission_control()));
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
    use sill_settings::CornerAction;

    use sill_launcher::WindowKey;
    use sill_model::ids::WorkspaceKey;
    use sill_model::windows::{Activity, Sizing, WindowState};
    use sill_model::windows::{Toplevel, WlCommand};

    use super::{CornerEffect, Stash, corner_effect, show_desktop};

    fn window(key: u64, workspace: u64, state: WindowState) -> Toplevel {
        Toplevel {
            key: WindowKey(key),
            app_id: format!("app{key}"),
            title: String::new(),
            activity: Activity::Inactive,
            window: state,
            sizing: Sizing::Floating,
            workspaces: vec![WorkspaceKey(workspace)],
            outputs: Vec::new(),
        }
    }

    #[test]
    fn show_desktop_minimises_the_workspace_then_brings_it_back() {
        let active = [WorkspaceKey(1)];
        let windows = vec![
            window(1, 1, WindowState::Normal),
            window(2, 1, WindowState::Minimized),
            window(3, 2, WindowState::Normal),
            window(4, 1, WindowState::Normal),
        ];
        let (commands, stash) = show_desktop(&windows, &active, &Stash::default());
        assert_eq!(
            commands,
            vec![
                WlCommand::Minimize(WindowKey(1)),
                WlCommand::Minimize(WindowKey(4))
            ]
        );
        assert_eq!(stash, Stash(vec![WindowKey(1), WindowKey(4)]));
        let hidden: Vec<_> = windows
            .into_iter()
            .map(|mut w| {
                if stash.0.contains(&w.key) {
                    w.window = WindowState::Minimized;
                }
                w
            })
            .collect();
        let (commands, after) = show_desktop(&hidden, &active, &stash);
        assert_eq!(
            commands,
            vec![
                WlCommand::Unminimize(WindowKey(1)),
                WlCommand::Unminimize(WindowKey(4))
            ]
        );
        assert_eq!(after, Stash::default());
        let restored_by_hand = show_desktop(&hidden[..1], &active, &Stash(vec![WindowKey(9)]));
        assert_eq!(
            restored_by_hand.1,
            Stash::default(),
            "a stash of gone windows starts over"
        );
    }

    #[test]
    fn each_action_has_its_effect() {
        let run = |program: &str, args: &[&str]| CornerEffect::Run {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        };
        let cases = [
            (CornerAction::None, "", CornerEffect::Nothing),
            (CornerAction::Launcher, "", CornerEffect::Launcher),
            (
                CornerAction::NotificationCenter,
                "",
                CornerEffect::NotificationCenter,
            ),
            (CornerAction::ControlCenter, "", CornerEffect::ControlCenter),
            (CornerAction::ShowDesktop, "", CornerEffect::ShowDesktop),
            (CornerAction::Lock, "", CornerEffect::Lock),
            (CornerAction::Workspaces, "", CornerEffect::MissionControl),
            (
                CornerAction::Command,
                "foot -e htop",
                run("/bin/sh", &["-c", "foot -e htop"]),
            ),
            (CornerAction::Command, "  ", CornerEffect::Nothing),
            (CornerAction::Launcher, "ignored", CornerEffect::Launcher),
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
