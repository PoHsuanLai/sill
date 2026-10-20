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
    AccountsSheet, CaptureSheet, ColorSampler, ConfirmSheet, DisplayTrialSheet, ForceQuit,
    HotCorners, IdleDim, LOCK, Osd, PermissionSheet, PolkitPrompt, PowerMenu, SpacesBar,
    SplitPicker, Switcher, VoiceConsentSheet, Wallpaper, WindowMenu,
};
use sill_settings::ShellFile;
use sill_ui_kit::links::Links;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surface::{MountLockProps, Surface, mount_lock};
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

/// One kind of surface, ready to spawn: where it draws and the call that spawns it.
#[derive(Clone, Copy)]
struct Kind {
    order: u8,
    spawn: fn(&mut HostCtx<ShellMsg>, &mut Outputs, &Spawn),
}

/// `S`, as a kind.
fn kind<S: Surface>() -> Kind {
    Kind {
        order: S::LAYER_ORDER,
        spawn: spawn_surface::<S>,
    }
}

/// Every kind of surface the shell spawns, in drawing order (`Surface::LAYER_ORDER`): the bar and
/// the wallpaper first, the overlays over them, the idle dim last, over every other Overlay
/// surface (the session-lock surfaces are above every layer surface anyway).
fn kinds() -> Vec<Kind> {
    let mut kinds = vec![
        kind::<SpacesBar>(),
        kind::<Bar>(),
        kind::<Wallpaper>(),
        kind::<Dock>(),
        kind::<LauncherCatcher>(),
        kind::<Launcher>(),
        kind::<PowerMenu>(),
        kind::<PolkitPrompt>(),
        kind::<ConfirmSheet>(),
        kind::<VoiceConsentSheet>(),
        kind::<AccountsSheet>(),
        kind::<CaptureSheet>(),
        kind::<PermissionSheet>(),
        kind::<DisplayTrialSheet>(),
        kind::<ColorSampler>(),
        kind::<ForceQuit>(),
        kind::<Switcher>(),
        kind::<Banners>(),
        kind::<NotificationCenter>(),
        kind::<ScreenshotThumb>(),
        kind::<HotCorners>(),
        kind::<WindowMenu>(),
        kind::<SplitPicker>(),
        kind::<Osd>(),
        kind::<IdleDim>(),
    ];
    kinds.sort_by_key(|kind| kind.order);
    kinds
}

/// Spawn whatever `outputs` lacks, in drawing order.
pub fn spawn_missing(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs, spawn: &Spawn) {
    prepare_wallpapers(outputs, spawn.services);
    for kind in kinds() {
        (kind.spawn)(ctx, outputs, spawn);
    }
    // A surface spawned while the service dims is mapped at once.
    crate::idle_dim::apply(ctx, outputs, spawn.services.idle.state.borrow().dim);
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

/// `root` with the contexts every surface the daemon spawns is given: the services and the
/// settings feed. The links come with each surface (`Surface::link`, `Surface::needs`). quire's
/// document host is not one of them: the root component installs it (`mount`, `mount_lock`),
/// the same call a popup's root makes. Every spawn goes through here, so a new surface cannot
/// forget the rest.
pub(crate) fn surface_root(
    root: HostRoot,
    services: &Services,
    settings_feed: &SettingsFeed,
) -> HostRoot {
    root.context(services.clone())
        .context(settings_feed.clone())
}

/// Each output's lock surface for shell-host's session lock: `LOCK`'s root with the
/// daemon's usual contexts and the links `LOCK.needs` names.
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
        config: (LOCK.config)(settings),
        build: Box::new(move |info: OutputInfo| {
            // The lock is not a `Surface` (the lock pipeline holds its surfaces), so it has its
            // own declaration of the links it reads: `LOCK.needs`.
            (LOCK.needs)(
                surface_root(
                    HostRoot::with_props(mount_lock, MountLockProps::new(LOCK, info)),
                    &services,
                    &settings_feed,
                ),
                &links,
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::kinds;

    #[test]
    fn every_kind_draws_at_a_place_of_its_own() {
        let orders: Vec<u8> = kinds().iter().map(|kind| kind.order).collect();
        assert_eq!(orders.len(), 25);
        assert!(
            orders.windows(2).all(|pair| pair[0] < pair[1]),
            "two kinds share a place: {orders:?}"
        );
    }
}
