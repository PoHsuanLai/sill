//! Hot corners on the daemon's side: the corner surfaces per output and per active corner, the
//! actions a firing carries out, and `sill debug corner`.
//!
//! A firing arrives on the [`CornerLink`] (from a corner surface's machine, or from
//! [`fake`]); [`spawn_hits`] follows it and carries out [`corner_effect`] of that corner's
//! action: the launcher, notification center and control center through their links, Lock
//! through the locker service (`crate::session::lock`), Show Desktop through the toplevel
//! manager ([`show_desktop`]), and the workspace overview and a custom command as child
//! processes.

use std::process::Stdio;

use shell_host::{HostCtx, OutputId, Root, SurfaceOp};
use sill_ipc::{CornerName, Failure, FailureKind, FakeCorner, Response};
use sill_launcher::WindowKey;
use sill_services::cosmic_wl::WorkspaceKey;
use sill_services::{Activity, Services, Toplevel, WindowState, WlCommand};
use sill_settings::{CornerAction, ShellFile};
use sill_surfaces::surfaces::hot_corners::{Corner, CornerLink, HotCornerProps, corner_layer};
use sill_surfaces::surfaces::launcher::LauncherLink;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// What a corner's action does, resolved from the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CornerEffect {
    Nothing,
    Launcher,
    NotificationCenter,
    ControlCenter,
    ShowDesktop,
    Lock,
    /// A child process: `cosmic-workspaces` for `Workspaces`, `/bin/sh -c COMMAND` for `Command`.
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
        CornerAction::Workspaces => CornerEffect::Run {
            program: "cosmic-workspaces".to_owned(),
            args: Vec::new(),
        },
        CornerAction::Command if command.trim().is_empty() => CornerEffect::Nothing,
        CornerAction::Command => CornerEffect::Run {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), command.to_owned()],
        },
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

/// The surface side's corner for the wire's name.
pub fn corner_of(name: CornerName) -> Corner {
    match name {
        CornerName::TopLeft => Corner::TopLeft,
        CornerName::TopRight => Corner::TopRight,
        CornerName::BottomLeft => Corner::BottomLeft,
        CornerName::BottomRight => Corner::BottomRight,
    }
}

/// Spawn a corner surface for each active corner of every output that lacks one, close the
/// surfaces of corners whose action became `None` (a settings change), and keep every remaining
/// one's size at `hot_corners.size_px`.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    root: impl Fn(Root) -> Root,
) {
    let file = settings.borrow().clone();
    let wanted = sill_surfaces::surfaces::hot_corners::active_corners(&file.hot_corners);
    let size = file.hot_corners.size_px.0;
    for (info, surfaces) in &mut outputs.outputs {
        for corner in Corner::ALL {
            match (wanted.contains(&corner), surfaces.corners.get(corner)) {
                (false, Some(id)) => {
                    ctx.close(id);
                    surfaces.corners.set(corner, None);
                }
                (true, None) => {
                    let config = corner_layer(info, corner, size);
                    let namespace = config.namespace.clone();
                    let props = HotCornerProps {
                        output: info.clone(),
                        corner,
                    };
                    let surface = root(Root::with_props(sill_surfaces::HotCorner, props));
                    match ctx.spawn_layer(config, surface) {
                        Ok(id) => surfaces.corners.set(corner, Some(id)),
                        Err(error) => {
                            eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name)
                        }
                    }
                }
                (true, Some(id)) => {
                    let config = corner_layer(info, corner, size);
                    ctx.apply(id, SurfaceOp::SetSize(config.size));
                }
                (false, None) => {}
            }
        }
    }
}

/// Follow `link` on `rt` and carry out each hit's action (read from `settings` at the hit)
/// through `services`, `links` and `launcher`.
pub fn spawn_hits(
    rt: &Handle,
    link: &CornerLink,
    settings: watch::Receiver<ShellFile>,
    services: Services,
    launcher: LauncherLink,
) {
    let mut hits = link.subscribe();
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
            let action =
                sill_surfaces::surfaces::hot_corners::corner_action(&file.hot_corners, hit.corner);
            let command =
                sill_surfaces::surfaces::hot_corners::corner_command(&file.hot_corners, hit.corner)
                    .to_owned();
            let effect = corner_effect(action, &command);
            act(effect, hit.output, &services, &launcher, &mut stash).await;
        }
    });
}

/// Carry out `effect` on `output`: toggle the launcher, notification center or control center;
/// minimise (or restore) windows for Show Desktop; lock through the session's locker; spawn a
/// detached child process for `Run` (the workspace overview, or a custom command).
async fn act(
    effect: CornerEffect,
    output: OutputId,
    services: &Services,
    launcher: &LauncherLink,
    stash: &mut Stash,
) {
    match effect {
        CornerEffect::Nothing => {}
        CornerEffect::Launcher => launcher.toggle(output),
        CornerEffect::NotificationCenter => {
            if let Some(links) = crate::links::get() {
                links.notification_center.toggle(output);
            }
        }
        CornerEffect::ControlCenter => {
            if let Some(links) = crate::links::get() {
                links.control_center.toggle(output);
            }
        }
        CornerEffect::ShowDesktop => {
            let windows = services.toplevels.borrow().windows.clone();
            let active: Vec<WorkspaceKey> = services
                .workspaces
                .borrow()
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
                let _ = services.wl.send(command);
            }
        }
        CornerEffect::Lock => {
            let _ = crate::session::lock(services);
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

/// `sill debug corner CORNER`: a hit of `corner` on the first output, as if its dwell had run
/// out.
pub fn fake(outputs: &Outputs, links: Option<&Links>, fake: FakeCorner) -> Response {
    let first = outputs.outputs.first().map(|(info, _)| info.id);
    match (links, first) {
        (Some(links), Some(output)) => {
            links.corners.hit(corner_of(fake.corner), output);
            Response::Done
        }
        (None, _) | (_, None) => Response::Failed(Failure {
            kind: FailureKind::Unsupported,
            message: "no output to fire a corner on".to_owned(),
        }),
    }
}

/// Every corner surface's namespace on `info`, with its corner, which `debug stats` reports.
pub fn namespaces(info: &shell_host::OutputInfo) -> Vec<(Corner, String)> {
    Corner::ALL
        .into_iter()
        .map(|corner| {
            let layer = sill_surfaces::surfaces::hot_corners::corner_layer(info, corner, 2);
            (corner, layer.namespace)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use sill_ipc::CornerName;
    use sill_settings::CornerAction;
    use sill_surfaces::surfaces::hot_corners::Corner;

    use sill_launcher::WindowKey;
    use sill_services::cosmic_wl::{Activity, Sizing, WindowState, WorkspaceKey};
    use sill_services::{Toplevel, WlCommand};

    use super::{CornerEffect, Stash, corner_effect, corner_of, show_desktop};

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
    fn wire_names_map_onto_corners() {
        assert_eq!(corner_of(CornerName::TopLeft), Corner::TopLeft);
        assert_eq!(corner_of(CornerName::BottomRight), Corner::BottomRight);
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
            (CornerAction::Workspaces, "", run("cosmic-workspaces", &[])),
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
