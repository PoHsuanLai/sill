//! The daemon's notifications half: one banner column and one notification center per output,
//! spawned warm and hidden (the OSD's and the power menu's pattern), and the banners' output.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::banners::{BannerOutput, BannersProps, banners_layer};
use sill_surfaces::surfaces::notification_center::{NotificationCenterProps, center_layer};
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden banner column and a hidden notification center for every output whose row
/// lacks one, each root built by `root` (the daemon's `surface_root`, which adds the links).
/// Called before the OSD's `spawn_missing`, so the OSD draws over a banner on the Overlay layer.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.banners.is_none() || s.notification_center.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let file = settings.borrow().clone();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        if surfaces.banners.is_none() {
            let props = BannersProps {
                output: info.clone(),
            };
            let root = root(Root::with_props(sill_surfaces::Banners, props));
            let config = banners_layer(&info, &file.notifications);
            surfaces.banners = crate::daemon::surfaces::spawn_hidden(ctx, config, root, &info);
        }
        if surfaces.notification_center.is_none() {
            let props = NotificationCenterProps {
                output: info.clone(),
            };
            let root = root(Root::with_props(sill_surfaces::NotificationCenter, props));
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
        .find(|(_, s)| s.banners.is_some())
        .map_or(BannerOutput::Nowhere, |(info, _)| BannerOutput::On(info.id))
}

/// The two surfaces' namespaces on `info`, which `debug stats` reports.
pub fn namespaces(info: &OutputInfo) -> (String, String) {
    (
        banners_layer(info, &Default::default()).namespace,
        center_layer(info).namespace,
    )
}

#[cfg(test)]
mod tests;
