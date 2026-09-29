//! Spawning the surfaces every output gets, each row's missing ones after every host event. The
//! bar, wallpaper, dock and launcher blocks live here; the other surfaces' `spawn_missing` sit in
//! their own modules and are called in their drawing order by [`spawn_missing`].

use shell_host::SurfaceId;
use shell_host::scale::{Size as BufferSize, buffer_size};
use shell_host::{HostCtx, OutputInfo, Root};
use sill_model::ids::OutputName;
use sill_model::wallpaper::WallpaperCommand;
use sill_settings::{SettingsFeed, ShellFile, WallpaperDrawer};
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::launcher::LauncherLink;
use sill_surfaces::surfaces::osd::OsdLink;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// What spawning a surface needs beside the outputs: the services and the settings every root
/// is given, and the links only some surfaces follow.
#[derive(Debug)]
pub struct Spawn<'a> {
    pub services: &'a Services,
    pub settings_feed: &'a SettingsFeed,
    pub settings: &'a watch::Sender<ShellFile>,
    /// The launcher's shared state; the panel's keyboard focus goes to it.
    pub launcher: &'a LauncherLink,
    /// What every output's OSD follows.
    pub osd: &'a OsdLink,
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
        launcher,
        osd,
    } = *spawn;
    let root = |root| surface_root(root, services, settings_feed);
    spawn_missing_bars(ctx, outputs, services, settings_feed, settings);
    spawn_missing_wallpapers(ctx, outputs, services, settings_feed, settings);
    spawn_missing_docks(ctx, outputs, services, settings_feed, settings);
    spawn_missing_launchers(ctx, outputs, services, settings_feed, settings, launcher);
    crate::power_menu::spawn_missing(ctx, outputs, root);
    // The polkit prompt over the power menu, and the switcher (both Overlay).
    crate::polkit::spawn_missing(ctx, outputs, root);
    crate::switcher::spawn_missing(ctx, outputs, root);
    // Before the OSD, so the OSD draws over a banner (both on the Overlay layer).
    crate::notifications::spawn_missing(ctx, outputs, settings, root);
    // The screenshot thumbnail after the banners and before the OSD (all Overlay); the
    // desktop widgets (Bottom) and the hot corners (Overlay, never painted).
    crate::screenshots::spawn_missing(ctx, outputs, root);
    crate::widgets::spawn_missing(ctx, outputs, settings, &services.dirs, root);
    crate::edit_widgets::spawn_missing(ctx, outputs, &services.dirs, root);
    crate::corners::spawn_missing(ctx, outputs, settings, root);
    crate::osd::spawn_missing(ctx, outputs, settings, osd, root);
    let dim = services.idle.state.borrow().dim;
    crate::idle_dim::spawn_missing(ctx, outputs, dim, root);
}

/// Spawn a bar for every output whose row has none yet.
fn spawn_missing_bars(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
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
        spawn_bar(ctx, outputs, services, settings_feed, &file, info);
    }
}

fn spawn_bar(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    file: &ShellFile,
    info: OutputInfo,
) {
    let config = sill_surfaces::surfaces::bar::bar_layer(&info, &file.bar);
    let props = sill_surfaces::surfaces::bar::BarProps {
        output: info.clone(),
    };
    let root = surface_root(
        Root::with_props(sill_surfaces::Bar, props),
        services,
        settings_feed,
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

/// Spawn a wallpaper for every output whose row has none yet (mirrors
/// [`spawn_missing_bars`]), unless `spaces.wallpaper_drawer` leaves the background to
/// `cosmic-bg`. Switching the key on takes effect at the next output event
/// or restart, and switching it off leaves any spawned wallpaper in place until restart.
fn spawn_missing_wallpapers(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    settings_tx: &watch::Sender<ShellFile>,
) {
    if settings_tx.borrow().spaces.wallpaper_drawer != WallpaperDrawer::Shell {
        return;
    }
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.wallpaper.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        spawn_wallpaper(ctx, outputs, services, settings_feed, info);
    }
}

/// One output's wallpaper layer (mirrors [`spawn_bar`]), and the `Prepare` that gives the
/// service the output's buffer size to resize into — without it the service never hears about
/// this output and the surface stays at `--paper` (design/20 §1.4). An unnamed output gets no
/// `Prepare` (there is no `sill_model::ids::OutputName` to key it by), the same way the bar's own
/// `on = output.name.clone().map(OutputName)` (`surfaces/bar/mod.rs`) leaves an unnamed output
/// at its default look rather than inventing a numeric stand-in nothing else keys by.
fn spawn_wallpaper(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    info: OutputInfo,
) {
    if let Some(name) = info.name.clone() {
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
    let config = sill_surfaces::surfaces::wallpaper::wallpaper_layer(&info);
    let props = sill_surfaces::surfaces::wallpaper::WallpaperProps {
        output: info.clone(),
    };
    let root = surface_root(
        Root::with_props(sill_surfaces::Wallpaper, props),
        services,
        settings_feed,
    );
    match ctx.spawn_layer(config, root) {
        Ok(id) => {
            let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
            surfaces.wallpaper = Some(id);
            outputs.set(info, surfaces);
        }
        Err(error) => eprintln!("sill: spawning the wallpaper on {:?}: {error}", info.name),
    }
}

/// Spawn a dock for every output whose row has none yet (the bar's pattern). The dock follows
/// later settings changes to its geometry itself (`surfaces::dock`).
fn spawn_missing_docks(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
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
        let config = sill_surfaces::surfaces::dock::dock_layer(&info, &file.dock);
        let props = sill_surfaces::surfaces::dock::DockProps {
            output: info.clone(),
        };
        let root = surface_root(
            Root::with_props(sill_surfaces::Dock, props),
            services,
            settings_feed,
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
    settings_tx: &watch::Sender<ShellFile>,
    launcher: &LauncherLink,
) {
    use sill_surfaces::surfaces::launcher::{
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
        let root =
            |root: Root| surface_root(root, services, settings_feed).context(launcher.clone());
        if surfaces.catcher.is_none() {
            let props = LauncherCatcherProps {
                output: output.clone(),
            };
            let root = root(Root::with_props(sill_surfaces::LauncherCatcher, props));
            surfaces.catcher = spawn_hidden(ctx, catcher_layer(&info), root, &info);
        }
        if surfaces.launcher.is_none() {
            let config = panel_layer(&info, &file.bar, &file.launcher);
            let root = root(Root::with_props(
                sill_surfaces::Launcher,
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
    root: Root,
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
/// quire's rect measurer, `ds_native::measure::MEASURE` (re-exported as `sill_surfaces::MEASURE`),
/// and its focus write, `ds_native::focus::FOCUS` (re-exported as `sill_surfaces::FOCUS`), and the
/// field caret, `ds_native::focus::CARET`, which are what `ds_native::measure::provide()` and
/// `ds_native::focus::provide()` provide from inside a root component. Without them a rect read or
/// a focus waits a frame and the panic hook prints the collision. Every spawn goes through here, so
/// a new surface cannot forget them.
pub(crate) fn surface_root(root: Root, services: &Services, settings_feed: &SettingsFeed) -> Root {
    let root = root
        .context(services.clone())
        .context(settings_feed.clone())
        .context(sill_surfaces::MEASURE)
        .context(sill_surfaces::FOCUS)
        .context(sill_surfaces::CARET);
    // The focus mode, the power menu and the control center's asks; the notification center and
    // the banners' output; the hot corners' firings; the switcher's asks and the lock prompt's
    // output (`crate::links`).
    match crate::links::get() {
        Some(links) => root
            .context(links.focus.clone())
            .context(links.power_menu.clone())
            .context(links.control_center.clone())
            .context(links.notification_center.clone())
            .context(links.banners.clone())
            .context(links.corners.clone())
            .context(links.switcher.clone())
            .context(links.lock.clone())
            .context(links.edit_widgets.clone())
            .context(links.widgets_writer.clone()),
        None => root,
    }
}

/// Each output's lock surface for shell-host's session lock: `LockScreen` with the
/// daemon's usual contexts and the lock prompt's link.
pub(crate) fn lock_factory(
    services: &Services,
    settings_feed: &SettingsFeed,
    link: &sill_surfaces::surfaces::lock::LockLink,
) -> crate::lock::host::LockFactory {
    let services = services.clone();
    let settings_feed = settings_feed.clone();
    let link = link.clone();
    crate::lock::host::LockFactory(Box::new(move |info: OutputInfo| {
        let props = sill_surfaces::surfaces::lock::LockScreenProps { output: info };
        surface_root(
            Root::with_props(sill_surfaces::LockScreen, props),
            &services,
            &settings_feed,
        )
        .context(link.clone())
    }))
}
