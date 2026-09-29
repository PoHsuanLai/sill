//! The daemon's notifications half: one notification center per output, spawned warm and hidden
//! (the power menu's pattern), and the banners' output. The banner column is spawned like any
//! `Surface` (`sill_ui_kit::Banners`).

use shell_host::{HostCtx, HostRoot, OutputInfo};
use sill_ui_kit::Banners;
use sill_ui_kit::links::BannerOutput;
use sill_ui_kit::surface::Surface;
use sill_ui_kit::surfaces::notification_center::{NotificationCenterProps, center_layer};

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden notification center for every output whose row lacks one, its root built by
/// `root` (the daemon's `surface_root`, which adds the links). Called after the banners'
/// spawn and before the OSD's, so the OSD draws over a banner on the Overlay layer.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(HostRoot) -> HostRoot,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.notification_center.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        if surfaces.notification_center.is_none() {
            let props = NotificationCenterProps {
                output: info.clone(),
            };
            let root = root(HostRoot::with_props(sill_ui_kit::NotificationCenter, props));
            surfaces.notification_center =
                crate::daemon::surfaces::spawn_hidden(ctx, center_layer(&info), root, &info);
        }
        outputs.set(info, surfaces);
    }
}

/// The output the banners show on: the first with a banner column (the main display's
/// stand-in, `banners/placement.rs`).
pub fn banner_output(outputs: &Outputs) -> BannerOutput {
    outputs
        .outputs
        .iter()
        .find(|(_, s)| s.spawned.iter().any(|kind| kind.name == Banners::NAME))
        .map_or(BannerOutput::Nowhere, |(info, _)| BannerOutput::On(info.id))
}

/// The notification center's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    center_layer(info).namespace
}

#[cfg(test)]
mod tests;
