//! Screenshots on the daemon's side: one warm, hidden thumbnail surface per output (the banners'
//! pattern), `sill screenshot`, `sill debug screenshot`, and the service's policy from
//! `screenshot.folders` and `screenshot.tool` ([`spawn_policy`]; until it is sent the service stays
//! passive and watches nothing).

use std::path::PathBuf;

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{FakeScreenshot, Response, ScreenshotRequest, ShotArea};
use sill_services::screenshots::{ShotPolicy, parse_folders, parse_tool};
use sill_services::{Control, Services, ShotCommand, ShotKind};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::screenshot_thumb::ScreenshotThumbProps;
use sill_surfaces::surfaces::screenshot_thumb::thumb_layer;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden thumbnail surface for every output whose row lacks one, its root built by
/// `root` (the daemon's `surface_root`). Called after the banners' and before the OSD's, so on
/// the Overlay layer the OSD draws over it.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.thumbnail.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = ScreenshotThumbProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::ScreenshotThumb, props));
        surfaces.thumbnail = crate::daemon::spawn_hidden(ctx, thumb_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The service's kind for the wire's.
pub fn kind_of(area: ShotArea) -> ShotKind {
    match area {
        ShotArea::Area => ShotKind::Area,
        ShotArea::Window => ShotKind::Window,
        ShotArea::Screen => ShotKind::Screen,
    }
}

/// `sill screenshot KIND`: ask the service to run the tool.
pub fn dispatch(services: &Services, request: ScreenshotRequest) -> Response {
    match request {
        ScreenshotRequest::Take(area) => send(services, ShotCommand::Take(kind_of(area))),
    }
}

/// `sill debug screenshot PATH`: as if the folder watch had just seen `PATH`.
pub fn fake(services: &Services, fake: FakeScreenshot) -> Response {
    send(services, ShotCommand::Announce(fake.path))
}

/// The service's policy `file` makes. Lane c fills it (`parse_folders` against `$HOME`,
/// `parse_tool`).
pub fn policy_of(file: &ShellFile) -> ShotPolicy {
    let home = std::env::var_os("HOME").map_or_else(PathBuf::new, PathBuf::from);
    ShotPolicy {
        folders: parse_folders(&file.screenshot.folders, &home),
        tool: parse_tool(&file.screenshot.tool),
    }
}

/// Send [`policy_of`] now and on every settings change, on `rt`, until either side is gone (the
/// notifications policy's pattern); a change elsewhere in the file sends nothing.
pub fn spawn_policy(
    rt: &Handle,
    mut settings: watch::Receiver<ShellFile>,
    control: Control<ShotCommand>,
) {
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let policy = policy_of(&settings.borrow_and_update());
            if sent.as_ref() != Some(&policy) {
                if control
                    .send(ShotCommand::Configure(policy.clone()))
                    .is_err()
                {
                    return;
                }
                sent = Some(policy);
            }
            if settings.changed().await.is_err() {
                return;
            }
        }
    });
}

/// The thumbnail surface's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    thumb_layer(info).namespace
}

fn send(services: &Services, command: ShotCommand) -> Response {
    match services.shots_control.send(command) {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(sill_ipc::Failure {
            kind: sill_ipc::FailureKind::Internal,
            message: format!("screenshots: {error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::ShotArea;
    use sill_services::ShotKind;
    use sill_services::screenshots::{ShotFolders, ShotPolicy, ShotTool};
    use sill_settings::ShellFile;

    use super::{kind_of, policy_of};

    #[test]
    fn wire_kinds_map_onto_the_services() {
        assert_eq!(kind_of(ShotArea::Area), ShotKind::Area);
        assert_eq!(kind_of(ShotArea::Window), ShotKind::Window);
        assert_eq!(kind_of(ShotArea::Screen), ShotKind::Screen);
    }

    #[test]
    fn the_default_settings_watch_pictures_with_cosmics_tool() {
        assert_eq!(
            policy_of(&ShellFile::default()),
            ShotPolicy {
                folders: ShotFolders::Auto,
                tool: ShotTool::Cosmic,
            }
        );
    }
}
