//! The desktop widget layer on the daemon's side (FINDINGS "M10 freeze"; M10 lane b fills the
//! stubs): one surface per output while `widgets.desktop` is `On` (closed when it turns `Off`),
//! and `sill widgets toggle-desktop`, which flips the key in `settings.toml` (the file stays the
//! one truth; the watch brings the change back as a `ShellMsg::Settings`).

use std::path::Path;

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{Failure, FailureKind, Response, WidgetsRequest};
use sill_settings::{DesktopWidgets, ShellFile};
use sill_surfaces::surfaces::desktop_widgets::desktop_layer;
use tokio::sync::watch;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn the desktop layer on every output that lacks one while `widgets.desktop` is `On`, and
/// close every one while it is `Off`. Lane b fills it; until then no desktop layer exists.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    root: impl Fn(Root) -> Root,
) {
    let _ = (ctx, outputs, settings, root);
}

/// `widgets.desktop` flipped.
pub fn flipped(now: DesktopWidgets) -> DesktopWidgets {
    match now {
        DesktopWidgets::Off => DesktopWidgets::On,
        DesktopWidgets::On => DesktopWidgets::Off,
    }
}

/// `sill widgets ...`. Lane b fills `ToggleDesktop` (load, flip with [`flipped`], save, and
/// publish on `settings`); until then it answers `Unsupported`.
pub fn dispatch(
    dir: Option<&Path>,
    settings: &watch::Sender<ShellFile>,
    request: WidgetsRequest,
) -> Response {
    let _ = (dir, settings);
    match request {
        WidgetsRequest::ToggleDesktop => Response::Failed(Failure {
            kind: FailureKind::Unsupported,
            message: "the desktop widget layer is not built yet (M10 lane b)".to_owned(),
        }),
    }
}

/// The desktop layer's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    desktop_layer(info).namespace
}

#[cfg(test)]
mod tests {
    use sill_settings::DesktopWidgets;

    use super::flipped;

    #[test]
    fn toggling_flips_the_key() {
        assert_eq!(flipped(DesktopWidgets::Off), DesktopWidgets::On);
        assert_eq!(flipped(DesktopWidgets::On), DesktopWidgets::Off);
    }
}
