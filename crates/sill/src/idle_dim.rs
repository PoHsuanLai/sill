//! The daemon's half of the idle dim: one dim surface per output, spawned warm and hidden, and
//! mapped only while the idle service publishes `Idle.dim = Dimmed(_)`. The surface itself follows
//! the same `Idle.dim` to fade in and snap out; this side only maps and unmaps, so an awake output
//! has no dim surface on screen at all.
//!
//! [`spawn_follower`] watches `services.idle` and sends `ShellMsg::Dim` whenever the dim
//! changes; [`apply`] sets every dim surface's visibility from it ([`visibility_of`]). A new
//! output's surface is spawned already mapped when the service is dimming.

use shell_host::{HostCtx, HostHandle, OutputInfo, Root, Visibility};
use sill_services::Services;
use sill_services::idle::DimShown;
use sill_surfaces::surfaces::idle_dim::{IdleDimOverlayProps, idle_dim_layer, visibility_of};
use tokio::runtime::Handle;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a dim surface for every output whose row has none yet, mapped only while `dim` dims.
/// `root` adds the contexts every surface gets.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    dim: DimShown,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.idle_dim.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let props = IdleDimOverlayProps {
            output: info.clone(),
        };
        let surface = root(Root::with_props(sill_surfaces::IdleDimOverlay, props));
        let id = crate::daemon::spawn_hidden(ctx, idle_dim_layer(&info), surface, &info);
        if let Some(id) = id {
            ctx.set_visibility(id, visibility_of(dim));
        }
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        surfaces.idle_dim = id;
        outputs.set(info, surfaces);
    }
}

/// Map every dim surface while `dim` dims, and unmap them all when it clears.
pub fn apply(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs, dim: DimShown) {
    let visibility: Visibility = visibility_of(dim);
    for id in outputs
        .outputs
        .iter()
        .filter_map(|(_, surfaces)| surfaces.idle_dim)
    {
        ctx.set_visibility(id, visibility);
    }
}

/// The dim surface's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    idle_dim_layer(info).namespace
}

/// Follow `services.idle` on `rt` and send `ShellMsg::Dim` to the host at start and whenever
/// the published dim changes, until the host is gone.
pub fn spawn_follower(rt: &Handle, services: &Services, host: HostHandle<ShellMsg>) {
    let mut idle = services.idle.clone();
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let dim = idle.borrow_and_update().dim;
            if sent != Some(dim) {
                if host.send(ShellMsg::Dim(dim)).is_err() {
                    return;
                }
                sent = Some(dim);
            }
            if idle.changed().await.is_err() {
                return;
            }
        }
    });
}
