//! The daemon's half of the idle dim: the dim surfaces (`sill_ui_kit::IdleDim`, one per output,
//! spawned warm and hidden) are mapped only while the idle service publishes `Idle.dim = Dimmed(_)`. The surface itself follows
//! the same `Idle.dim` to fade in and snap out; this side only maps and unmaps, so an awake output
//! has no dim surface on screen at all.
//!
//! [`spawn_follower`] watches `services.idle.state` and sends `ShellMsg::Dim` whenever the dim
//! changes; [`apply`] sets every dim surface's visibility from it ([`visibility_of`]). A new
//! output's surface is mapped at once when the service is dimming ([`crate::daemon::surfaces`]
//! calls [`apply`] after spawning).

use shell_host::{HostCtx, HostHandle, Visibility};
use sill_model::idle::DimShown;
use sill_ui_kit::IdleDim;
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::Surface;
use sill_ui_kit::surfaces::idle_dim::visibility_of;
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Map every dim surface while `dim` dims, and unmap them all when it clears.
pub fn apply(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs, dim: DimShown) {
    let visibility: Visibility = visibility_of(dim);
    for surface in outputs
        .outputs
        .iter()
        .flat_map(|(_, surfaces)| &surfaces.spawned)
        .filter(|surface| surface.name == IdleDim::NAME)
    {
        ctx.set_visibility(surface.id, visibility);
    }
}

/// Follow `services.idle.state` on `rt` and send `ShellMsg::Dim` to the host at start and whenever
/// the published dim changes, until the host is gone.
pub fn spawn_follower(rt: &Handle, services: &Services, host: HostHandle<ShellMsg>) {
    let mut idle = services.idle.state.clone();
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
