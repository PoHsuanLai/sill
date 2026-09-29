//! Screenshots on the daemon's side: one warm, hidden thumbnail surface per output (the banners'
//! pattern).

use shell_host::{HostCtx, HostRoot, OutputInfo};
use sill_surfaces::surfaces::screenshot_thumb::ScreenshotThumbProps;
use sill_surfaces::surfaces::screenshot_thumb::thumb_layer;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden thumbnail surface for every output whose row lacks one, its root built by
/// `root` (the daemon's `surface_root`). Called after the banners' and before the OSD's, so on
/// the Overlay layer the OSD draws over it.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(HostRoot) -> HostRoot,
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
        let root = root(HostRoot::with_props(sill_surfaces::ScreenshotThumb, props));
        surfaces.thumbnail =
            crate::daemon::surfaces::spawn_hidden(ctx, thumb_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The thumbnail surface's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    thumb_layer(info).namespace
}
