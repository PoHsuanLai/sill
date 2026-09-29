//! Spawning the surfaces every output gets, each row's missing ones after every host event. The
//! bar, wallpaper, dock and launcher blocks live here; the other surfaces' `spawn_missing` sit in
//! their own modules and are called in their drawing order by [`spawn_missing`].

use shell_host::SurfaceId;
use shell_host::{HostCtx, HostRoot, OutputInfo};
use shell_host::{Size as BufferSize, buffer_size};
use sill_model::ids::OutputName;
use sill_model::wallpaper::WallpaperCommand;
use sill_settings::{SettingsFeed, ShellFile, WallpaperDrawer};
use sill_ui_kit::links::Links;
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::{LockSurface, MountLockProps, Surface, mount_lock};
use sill_ui_kit::{HotCorners, IdleDim, Lock, Osd, PolkitPrompt, PowerMenu, Switcher, Wallpaper};
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::daemon::spawn::spawn_surface;
use crate::outputs::Outputs;

/// What spawning a surface needs beside the outputs: the services, the settings and the links
/// every root is given.
#[derive(Debug)]
pub struct Spawn<'a> {
    pub services: &'a Services,
    pub settings_feed: &'a SettingsFeed,
    pub settings: &'a watch::Sender<ShellFile>,
    pub links: &'a Links,
}

/// Spawn whatever `outputs` lacks, in drawing order: the bar, wallpaper, dock and launcher, then
/// the overlays. The OSD comes after the banners and the screenshot thumbnail so it draws over
/// them, and the idle dim last, over every other Overlay surface (the session-lock surfaces are
/// above every layer surface anyway).
pub fn spawn_missing(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs, spawn: &Spawn) {
    let Spawn {
        services,
        settings_feed,
        settings,
        links,
    } = *spawn;
    let root = |root| surface_root(root, services, settings_feed, links);
    spawn_missing_bars(ctx, outputs, services, settings_feed, links, settings);
    prepare_wallpapers(outputs, services, &settings.borrow());
    spawn_surface::<Wallpaper>(ctx, outputs, spawn, &());
    spawn_missing_docks(ctx, outputs, services, settings_feed, links, settings);
    spawn_missing_launchers(ctx, outputs, services, settings_feed, links, settings);
    spawn_surface::<PowerMenu>(ctx, outputs, spawn, &links.power_menu);
    // The polkit prompt over the power menu, and the switcher (both Overlay).
    spawn_surface::<PolkitPrompt>(ctx, outputs, spawn, &links.banners);
    spawn_surface::<Switcher>(ctx, outputs, spawn, &links.switcher);
    // Before the OSD, so the OSD draws over a banner (both on the Overlay layer).
    crate::notifications::spawn_missing(ctx, outputs, settings, root);
    // The screenshot thumbnail after the banners and before the OSD (all Overlay); the
    // desktop widgets (Bottom) and the hot corners (Overlay, never painted).
    crate::screenshots::spawn_missing(ctx, outputs, root);
    crate::widgets::spawn_missing(ctx, outputs, settings, &services.dirs, root);
    crate::edit_widgets::spawn_missing(ctx, outputs, &services.dirs, root);
    spawn_surface::<HotCorners>(ctx, outputs, spawn, &links.corners);
    spawn_surface::<Osd>(ctx, outputs, spawn, &links.osd);
    spawn_surface::<IdleDim>(ctx, outputs, spawn, &());
    // A surface spawned while the service dims is mapped at once.
    crate::idle_dim::apply(ctx, outputs, services.idle.state.borrow().dim);
}

/// Spawn a bar for every output whose row has none yet.
fn spawn_missing_bars(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
    settings_tx: &watch::Sender<ShellFile>,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.bar.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    if missing.is_empty() {
        return;
    }
    let file = settings_tx.borrow().clone();
    for info in missing {
        spawn_bar(ctx, outputs, services, settings_feed, links, &file, info);
    }
}

fn spawn_bar(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
    file: &ShellFile,
    info: OutputInfo,
) {
    let config = sill_ui_kit::surfaces::bar::bar_layer(&info, &file.bar);
    let props = sill_ui_kit::surfaces::bar::BarProps {
        output: info.clone(),
    };
    let root = surface_root(
        HostRoot::with_props(sill_ui_kit::Bar, props),
        services,
        settings_feed,
        links,
    );
    match ctx.spawn_layer(config, root) {
        Ok(id) => {
            let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
            surfaces.bar = Some(id);
            outputs.set(info, surfaces);
        }
        Err(error) => eprintln!("sill: spawning the bar on {:?}: {error}", info.name),
    }
}

/// Give the wallpaper service the buffer size of every output that is about to get a wallpaper
/// surface: without a `Prepare` the service never hears about the output and the surface stays
/// at `--paper` (design/20 §1.4). An unnamed output gets none (there is no
/// `sill_model::ids::OutputName` to key it by), the same way the bar's own
/// `on = output.name.clone().map(OutputName)` leaves an unnamed output at its default look.
fn prepare_wallpapers(outputs: &Outputs, services: &Services, settings: &ShellFile) {
    if settings.spaces.wallpaper_drawer != WallpaperDrawer::Shell {
        return;
    }
    for (info, row) in &outputs.outputs {
        let held = row.spawned.iter().any(|s| s.name == Wallpaper::NAME);
        let Some(name) = info.name.clone().filter(|_| !held) else {
            continue;
        };
        let buffer = buffer_size(
            BufferSize {
                width: info.logical.width,
                height: info.logical.height,
            },
            info.scale,
        );
        let prepared = services.wallpaper.control.send(WallpaperCommand::Prepare {
            output: OutputName(name),
            width: buffer.width.unsigned_abs(),
            height: buffer.height.unsigned_abs(),
        });
        if let Err(error) = prepared {
            eprintln!("sill: preparing the wallpaper on {:?}: {error}", info.name);
        }
    }
}

/// Spawn a dock for every output whose row has none yet (the bar's pattern). The dock follows
/// later settings changes to its geometry itself (`surfaces::dock`).
fn spawn_missing_docks(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
    settings_tx: &watch::Sender<ShellFile>,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.dock.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let file = settings_tx.borrow().clone();
    for info in missing {
        let config = sill_ui_kit::surfaces::dock::dock_layer(&info, &file.dock);
        let props = sill_ui_kit::surfaces::dock::DockProps {
            output: info.clone(),
        };
        let root = surface_root(
            HostRoot::with_props(sill_ui_kit::Dock, props),
            services,
            settings_feed,
            links,
        );
        match ctx.spawn_layer(config, root) {
            Ok(id) => {
                let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
                surfaces.dock = Some(id);
                outputs.set(info, surfaces);
            }
            Err(error) => eprintln!("sill: spawning the dock on {:?}: {error}", info.name),
        }
    }
}

/// Spawn the launcher's two surfaces for every output whose row has none yet, both hidden at once:
/// they stay alive and are shown and hidden with `set_visibility`, following `launcher`. The
/// catcher is spawned first.
fn spawn_missing_launchers(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
    settings_tx: &watch::Sender<ShellFile>,
) {
    use sill_ui_kit::surfaces::launcher::{
        LauncherCatcherProps, LauncherProps, catcher_layer, panel_layer,
    };
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.launcher.is_none() || s.catcher.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let file = settings_tx.borrow().clone();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let output = info.clone();
        let root = |root: HostRoot| {
            surface_root(root, services, settings_feed, links).context(links.launcher.clone())
        };
        if surfaces.catcher.is_none() {
            let props = LauncherCatcherProps {
                output: output.clone(),
            };
            let root = root(HostRoot::with_props(sill_ui_kit::LauncherCatcher, props));
            surfaces.catcher = spawn_hidden(ctx, catcher_layer(&info), root, &info);
        }
        if surfaces.launcher.is_none() {
            let config = panel_layer(&info, &file.bar, &file.launcher);
            let root = root(HostRoot::with_props(
                sill_ui_kit::Launcher,
                LauncherProps { output },
            ));
            surfaces.launcher = spawn_hidden(ctx, config, root, &info);
        }
        outputs.set(info, surfaces);
    }
}

/// Spawn a layer surface and hide it before its first paint.
pub(crate) fn spawn_hidden(
    ctx: &mut HostCtx<ShellMsg>,
    config: shell_host::LayerConfig,
    root: HostRoot,
    info: &OutputInfo,
) -> Option<SurfaceId> {
    let namespace = config.namespace.clone();
    match ctx.spawn_layer(config, root) {
        Ok(id) => {
            ctx.set_visibility(id, shell_host::Visibility::Hidden);
            Some(id)
        }
        Err(error) => {
            eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name);
            None
        }
    }
}

/// `root` with the contexts every surface the daemon spawns is given: the services, the settings,
/// quire's rect measurer, `ds_native::measure::MEASURE` (re-exported as `sill_ui_kit::root::MEASURE`),
/// and its focus write, `ds_native::focus::FOCUS` (re-exported as `sill_ui_kit::root::FOCUS`), and the
/// field caret, `ds_native::focus::CARET`, which are what `ds_native::measure::provide()` and
/// `ds_native::focus::provide()` provide from inside a root component. Without them a rect read or
/// a focus waits a frame and the panic hook prints the collision. Every spawn goes through here, so
/// a new surface cannot forget them.
pub(crate) fn surface_root(
    root: HostRoot,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
) -> HostRoot {
    let root = root
        .context(services.clone())
        .context(settings_feed.clone())
        .context(sill_ui_kit::root::MEASURE)
        .context(sill_ui_kit::root::FOCUS)
        .context(sill_ui_kit::root::CARET);
    // The focus mode, the power menu and the control center's asks; the notification center and
    // the banners' output; the switcher's asks and the lock prompt's
    // output (`sill_ui_kit::links`).
    root.context(links.focus.clone())
        .context(links.power_menu.clone())
        .context(links.control_center.clone())
        .context(links.notification_center.clone())
        .context(links.banners.clone())
        .context(links.switcher.clone())
        .context(links.lock.clone())
        .context(links.edit_widgets.clone())
        .context(links.widgets_writer.clone())
}

/// Each output's lock surface for shell-host's session lock: `Lock`'s root with the
/// daemon's usual contexts, which include the lock prompt's link.
pub(crate) fn lock_factory(
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
    settings: &ShellFile,
) -> crate::lock::host::LockFactory {
    let services = services.clone();
    let settings_feed = settings_feed.clone();
    let links = links.clone();
    crate::lock::host::LockFactory {
        config: Lock::config(settings),
        build: Box::new(move |info: OutputInfo| {
            surface_root(
                HostRoot::with_props(mount_lock::<Lock>, MountLockProps::new(info)),
                &services,
                &settings_feed,
                &links,
            )
        }),
    }
}
