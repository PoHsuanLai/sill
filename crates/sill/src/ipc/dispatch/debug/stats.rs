//! `sill debug stats`: every surface's frame statistics, through the always-public
//! `HostCtx::take_stats` (it needs neither the `debug` feature nor shell-host's `debug-inject`;
//! draining, so each report covers only the time since the previous one).

use shell_host::{HostCtx, OutputInfo, SurfaceId};
use sill_ipc::debug::{Micros, SurfaceName, SurfaceStats};

use crate::daemon::msg::ShellMsg;
use crate::outputs::{OutputSurfaces, Outputs, bar_namespace, wallpaper_namespace};

/// The dock's layer namespace on `info` (`sill-dock-<output>`), which `debug stats` reports.
fn dock_namespace(info: &OutputInfo) -> String {
    sill_ui_kit::surfaces::dock::dock_layer(info, &sill_settings::DockSettings::default()).namespace
}

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
            let label = format!("{}/{name}", crate::lock::host::LOCK_NAMESPACE);
            let id = ctx.surface_named(&shell_host::SurfaceName(label.clone()))?;
            Some((label, id))
        })
        .collect()
}

/// One output's spawned surfaces, each with its layer namespace.
fn row_surfaces(info: &OutputInfo, surfaces: &OutputSurfaces) -> Vec<(String, SurfaceId)> {
    let bar = surfaces.bar.map(|id| (bar_namespace(info), id));
    let wallpaper = surfaces.wallpaper.map(|id| (wallpaper_namespace(info), id));
    let dock = surfaces.dock.map(|id| (dock_namespace(info), id));
    let (panel, catcher) = launcher_namespaces(info);
    let launcher = surfaces.launcher.map(|id| (panel, id));
    let catcher = surfaces.catcher.map(|id| (catcher, id));
    let power_menu = surfaces.power_menu.map(|id| {
        let layer = sill_ui_kit::surfaces::power_menu::power_menu_layer(info);
        (layer.namespace, id)
    });
    let (banners_name, center_name) = crate::notifications::namespaces(info);
    let banners = surfaces.banners.map(|id| (banners_name, id));
    let center = surfaces.notification_center.map(|id| (center_name, id));
    let thumbnail = surfaces
        .thumbnail
        .map(|id| (crate::screenshots::namespace(info), id));
    let desktop = surfaces
        .desktop_widgets
        .map(|id| (crate::widgets::namespace(info), id));
    let edit = surfaces
        .edit_widgets
        .map(|id| (crate::edit_widgets::namespace(info), id));
    let polkit = surfaces
        .polkit_prompt
        .map(|id| (crate::polkit::namespace(info), id));
    let switcher = surfaces
        .switcher
        .map(|id| (crate::switcher::namespace(info), id));
    let idle_dim = surfaces
        .idle_dim
        .map(|id| (crate::idle_dim::namespace(info), id));
    let corners: Vec<_> = crate::corners::namespaces(info)
        .into_iter()
        .filter_map(|(corner, name)| surfaces.corners.get(corner).map(|id| (name, id)))
        .collect();
    let spawned = surfaces
        .spawned
        .iter()
        .map(|surface| (surface.namespace.clone(), surface.id));
    bar.into_iter()
        .chain(wallpaper)
        .chain(dock)
        .chain(launcher)
        .chain(catcher)
        .chain(power_menu)
        .chain(banners)
        .chain(center)
        .chain(thumbnail)
        .chain(desktop)
        .chain(edit)
        .chain(corners)
        .chain(polkit)
        .chain(switcher)
        .chain(idle_dim)
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
