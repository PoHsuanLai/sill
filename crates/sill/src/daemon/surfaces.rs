//! Spawning the surfaces every output gets, each row's missing ones after every host event, all
//! through `spawn_surface` in their drawing order ([`spawn_missing`]). The wallpaper's `Prepare`
//! for the wallpaper service lives here.

use shell_host::SurfaceId;
use shell_host::{HostCtx, HostRoot, OutputInfo};
use shell_host::{Size as BufferSize, buffer_size};
use sill_bar::Bar;
use sill_dock::Dock;
use sill_launcher_ui::{Launcher, LauncherCatcher};
use sill_model::ids::OutputName;
use sill_model::wallpaper::WallpaperCommand;
use sill_notify_ui::{Banners, NotificationCenter, ScreenshotThumb};
use sill_overlays::{
    AccountsSheet, CaptureSheet, ConfirmSheet, HotCorners, IdleDim, Lock, Osd, PolkitPrompt,
    PowerMenu, SplitPicker, Switcher, VoiceConsentSheet, Wallpaper, WindowMenu,
};
use sill_settings::ShellFile;
use sill_ui_kit::links::Links;
use sill_ui_kit::links::PlacementFile;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surface::{LockSurface, MountLockProps, mount_lock};
use sill_widgets_ui::{DesktopWidgets, EditWidgets};
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
        services, links, ..
    } = *spawn;
    spawn_surface::<Bar>(ctx, outputs, spawn, &links.control_center);
    prepare_wallpapers(outputs, services);
    spawn_surface::<Wallpaper>(ctx, outputs, spawn, &());
    spawn_surface::<Dock>(ctx, outputs, spawn, &());
    spawn_surface::<LauncherCatcher>(ctx, outputs, spawn, &links.launcher);
    spawn_surface::<Launcher>(ctx, outputs, spawn, &links.launcher);
    spawn_surface::<PowerMenu>(ctx, outputs, spawn, &links.power_menu);
    // The polkit prompt over the power menu, and the switcher (both Overlay).
    spawn_surface::<PolkitPrompt>(ctx, outputs, spawn, &links.banners);
    // The confirmation sheet beside it: both ask the person, on the banners' output.
    spawn_surface::<ConfirmSheet>(ctx, outputs, spawn, &links.banners);
    // The voice consent sheet asks the same way, once, before the microphone first opens.
    spawn_surface::<VoiceConsentSheet>(ctx, outputs, spawn, &links.banners);
    // accountd's consent question and add-account steps, a window of their own on the same output.
    spawn_surface::<AccountsSheet>(ctx, outputs, spawn, &links.banners);
    // The screen-sharing picker, on the same output, when an app asks what to share.
    spawn_surface::<CaptureSheet>(ctx, outputs, spawn, &links.banners);
    spawn_surface::<Switcher>(ctx, outputs, spawn, &links.switcher);
    // Before the OSD, so the OSD draws over a banner (both on the Overlay layer).
    spawn_surface::<Banners>(ctx, outputs, spawn, &links.banners);
    spawn_surface::<NotificationCenter>(ctx, outputs, spawn, &links.notification_center);
    // The screenshot thumbnail after the banners and before the OSD (all Overlay); the
    // desktop widgets (Bottom) and the hot corners (Overlay, never painted).
    spawn_surface::<ScreenshotThumb>(ctx, outputs, spawn, &links.banners);
    spawn_surface::<DesktopWidgets>(ctx, outputs, spawn, &PlacementFile::in_dirs(&services.dirs));
    spawn_surface::<EditWidgets>(ctx, outputs, spawn, &links.edit_widgets);
    spawn_surface::<HotCorners>(ctx, outputs, spawn, &links.corners);
    // The window menu's transparent parent (Overlay, takes no input): the popup the green light
    // opens hangs from it.
    spawn_surface::<WindowMenu>(ctx, outputs, spawn, &links.window_menu);
    // The Split View picker (hidden until the compositor offers the other half).
    spawn_surface::<SplitPicker>(ctx, outputs, spawn, &links.split_picker);
    spawn_surface::<Osd>(ctx, outputs, spawn, &links.osd);
    spawn_surface::<IdleDim>(ctx, outputs, spawn, &());
    // A surface spawned while the service dims is mapped at once.
    crate::idle_dim::apply(ctx, outputs, services.idle.state.borrow().dim);
}

/// Give the wallpaper service the buffer size of every output that is about to get a wallpaper
/// surface: without a `Prepare` the service never hears about the output and the surface stays
/// at `--paper` (design/20 §1.4). An unnamed output gets none (there is no
/// `sill_model::ids::OutputName` to key it by), the same way the bar's own
/// `on = output.name.clone().map(OutputName)` leaves an unnamed output at its default look.
fn prepare_wallpapers(outputs: &Outputs, services: &Services) {
    for (info, row) in &outputs.outputs {
        let held = row.surface_of::<Wallpaper>().is_some();
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

/// `root` with the contexts every surface the daemon spawns is given: the services, the settings
/// feed and the links. quire's document host is not one of them: the root component installs it
/// (`mount`, `mount_lock`), the same call a popup's root makes. Every spawn goes through here, so
/// a new surface cannot forget the rest.
pub(crate) fn surface_root(
    root: HostRoot,
    services: &Services,
    settings_feed: &SettingsFeed,
    links: &Links,
) -> HostRoot {
    let root = root
        .context(services.clone())
        .context(settings_feed.clone());
    // The focus mode, the power menu and the control center's asks; the notification center and
    // the banners' output; the switcher's asks and the lock prompt's
    // output (`sill_ui_kit::links`).
    root.context(links.focus.clone())
        .context(links.power_menu.clone())
        .context(links.notification_center.clone())
        .context(links.banners.clone())
        .context(links.switcher.clone())
        .context(links.lock.clone())
        .context(links.edit_widgets.clone())
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
