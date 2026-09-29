//! `sill debug stats`: every surface's frame statistics, through the always-public
//! `HostCtx::take_stats` (it needs neither the `debug` feature nor shell-host's `debug-inject`;
//! draining, so each report covers only the time since the previous one).

use shell_host::{HostCtx, OutputInfo, SurfaceId};
use sill_ipc::debug::{Micros, SurfaceName, SurfaceStats};
use sill_settings::ShellFile;
use sill_ui_kit::Lock;
use sill_ui_kit::surface::LockSurface;

use crate::daemon::msg::ShellMsg;
use crate::outputs::{OutputSurfaces, Outputs, bar_namespace};

/// The launcher panel's and catcher's namespaces on `info`, which `debug stats` reports.
fn launcher_namespaces(info: &OutputInfo) -> (String, String) {
    use sill_ui_kit::surfaces::launcher::{catcher_layer, panel_layer};
    let panel = panel_layer(info, &Default::default(), &Default::default()).namespace;
    (panel, catcher_layer(info).namespace)
}

/// Every known surface's frame statistics: the rows' surfaces, and the lock surfaces while a lock
/// is up.
pub fn collect(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<SurfaceStats> {
    let ids: Vec<(String, SurfaceId)> = outputs
        .outputs
        .iter()
        .flat_map(|(info, surfaces)| row_surfaces(info, surfaces))
        .collect();
    let locks = lock_surfaces(ctx, outputs);
    ids.into_iter()
        .chain(locks)
        .filter_map(|(name, id)| Some(wire_stats(name, ctx.take_stats(id)?)))
        .collect()
}

/// The lock surfaces are the host's (shell-host names each `sill-lock/<output>`), not a row of
/// `outputs`, so they are found by name.
fn lock_surfaces(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<(String, SurfaceId)> {
    outputs
        .outputs
        .iter()
        .filter_map(|(info, _)| info.name.clone())
        .filter_map(|name| {
            let namespace = <Lock as LockSurface>::config(&ShellFile::default()).namespace;
            let label = format!("{namespace}/{name}");
            let id = ctx.surface_named(&shell_host::SurfaceName(label.clone()))?;
            Some((label, id))
        })
        .collect()
}

/// One output's spawned surfaces, each with its layer namespace.
fn row_surfaces(info: &OutputInfo, surfaces: &OutputSurfaces) -> Vec<(String, SurfaceId)> {
    let bar = surfaces.bar.map(|id| (bar_namespace(info), id));
    let (panel, catcher) = launcher_namespaces(info);
    let launcher = surfaces.launcher.map(|id| (panel, id));
    let catcher = surfaces.catcher.map(|id| (catcher, id));
    let desktop = surfaces
        .desktop_widgets
        .map(|id| (crate::widgets::namespace(info), id));
    let edit = surfaces
        .edit_widgets
        .map(|id| (crate::edit_widgets::namespace(info), id));
    let spawned = surfaces
        .spawned
        .iter()
        .map(|surface| (surface.namespace.clone(), surface.id));
    bar.into_iter()
        .chain(launcher)
        .chain(catcher)
        .chain(desktop)
        .chain(edit)
        .chain(spawned)
        .collect()
}

fn wire_stats(name: String, stats: shell_host::FrameStats) -> SurfaceStats {
    SurfaceStats {
        surface: SurfaceName(name),
        frames: stats.counters.frames,
        callbacks: stats.counters.callbacks,
        commits: stats.counters.commits,
        skipped: stats.counters.skipped,
        watchdog_fired: stats.counters.watchdog_fired,
        work: wire_micros(stats.work),
        interval: wire_micros(stats.interval),
        samples: stats.samples,
        vm_rss_kib: stats.vm_rss_kib,
    }
}

fn wire_micros(p: shell_host::Percentiles) -> Micros {
    Micros {
        p50_us: p.p50_us,
        p95_us: p.p95_us,
        max_us: p.max_us,
    }
}
