//! Screenshots on the daemon's side: one warm, hidden thumbnail surface per output (the banners'
//! pattern), `sill screenshot`, `sill debug screenshot`, and the service's policy from
//! `screenshot.folders` and `screenshot.tool` ([`Bridge`]; until it is sent the service stays
//! passive and watches nothing).

use std::path::{Path, PathBuf};

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{FakeScreenshot, Response, ScreenshotRequest, ShotArea};
use sill_model::screenshots::{ShotCommand, ShotKind, ShotPolicy, parse_folders, parse_tool};
use sill_services::Services;
use sill_settings::ShellFile;
use sill_surfaces::surfaces::screenshot_thumb::ScreenshotThumbProps;
use sill_surfaces::surfaces::screenshot_thumb::thumb_layer;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;
use crate::policy::Policy;

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

/// The service's policy `file` makes (`parse_folders` against `home`, `parse_tool`).
pub fn policy_of(file: &ShellFile, home: &Path) -> ShotPolicy {
    ShotPolicy {
        folders: parse_folders(&file.screenshot.folders, home),
        tool: parse_tool(&file.screenshot.tool),
    }
}

/// The screenshots service's policy from the settings, its folders found against `home`.
#[derive(Debug, Clone)]
pub struct Bridge {
    pub home: PathBuf,
}

impl Policy for Bridge {
    type Command = ShotCommand;
    type Out = ShotPolicy;

    fn of(&self, file: &ShellFile) -> ShotPolicy {
        policy_of(file, &self.home)
    }

    fn command(policy: ShotPolicy) -> ShotCommand {
        ShotCommand::Configure(policy)
    }
}

/// The thumbnail surface's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    thumb_layer(info).namespace
}

fn send(services: &Services, command: ShotCommand) -> Response {
    match services.screenshots.control.send(command) {
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
    use sill_model::screenshots::ShotKind;
    use sill_model::screenshots::{ShotFolders, ShotPolicy, ShotTool};
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
            policy_of(&ShellFile::default(), std::path::Path::new("/home/p")),
            ShotPolicy {
                folders: ShotFolders::Auto,
                tool: ShotTool::Cosmic,
            }
        );
    }
}
