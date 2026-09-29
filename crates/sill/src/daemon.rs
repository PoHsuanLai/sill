//! `sill run`: take the single-instance lock, start the services and the IPC server, connect the
//! host, and keep one set of surfaces per output. The lock is
//! `latchkey::Agent::new("sill").listen()`; `AlreadyRunning` exits 0.
//!
//! [`on_event`] is the frozen event handler; it owns the parts that need only `HostCtx` and
//! [`Outputs`] (output bookkeeping via [`crate::outputs::reconcile`], closing a surface the
//! compositor took away, resizing the bar when settled settings say to). Spawning a surface
//! needs [`sill_surfaces::services::Services`] and the live settings, neither of which fits `on_event`'s
//! signature, so [`run`] wraps it: [`run_event`] answers IPC requests (which also need
//! `Services`) and, after delegating every other event to `on_event`, spawns whatever `on_event`
//! marked as missing (an `OutputSurfaces` row with `bar: None`).

pub mod msg;
pub(crate) mod services;
pub(crate) mod settings;

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use shell_host::scale::{Size as BufferSize, buffer_size};
use shell_host::{CloseReason, Host, HostConfig, HostCtx, HostEvent, OutputInfo, Root, SurfaceId};
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_ipc::show::{LauncherRequest, ShowRequest};
use sill_model::ids::OutputName;
use sill_model::locker::LockerCommand;
use sill_model::polkit_agent::{AgentPolicy, PolkitCommand};
use sill_model::wallpaper::WallpaperCommand;
use sill_settings::{SettingsFeed, ShellFile, WallpaperDrawer};
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::launcher::LauncherLink;
use sill_surfaces::surfaces::osd::OsdLink;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::ipc::dispatch::{Reach, route};
use crate::ipc::{latency, server};
use crate::outputs::{OutputAction, OutputSurfaces, Outputs, reconcile};
use crate::policy::follow;

/// Run the daemon until `quit` or a fatal host error, on the seams `args` make.
pub fn run(args: &crate::run_args::RunArgs) -> ExitCode {
    let agent = match latchkey::Agent::new("sill") {
        Ok(agent) => agent,
        Err(error) => return fatal(&format!("naming this agent: {error}")),
    };
    let door = match agent.listen() {
        Ok(door) => door,
        Err(latchkey::Error::AlreadyRunning) => return ExitCode::SUCCESS,
        Err(error) => return fatal(&format!("taking the single-instance lock: {error}")),
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return fatal(&format!("starting the async runtime: {error}")),
    };

    let env = args.env();
    let services = services::start(runtime.handle(), &env);

    let settings_dir = Some(env.dirs.sill_config());
    let initial_settings = settings_dir
        .as_deref()
        .map(sill_settings::load)
        .unwrap_or_default();
    let (settings_tx, settings_rx) = watch::channel(initial_settings.clone());
    let settings_feed = SettingsFeed(settings_rx);
    crate::links::install(crate::links::Links {
        focus: sill_settings::spawn_focus(
            runtime.handle(),
            settings_feed.0.clone(),
            settings_dir.clone(),
        ),
        power_menu: sill_surfaces::surfaces::power_menu::PowerMenuLink::new(),
        control_center: sill_surfaces::surfaces::control_center::ControlCenterLink::new(),
        notification_center: sill_surfaces::surfaces::notification_center::CenterLink::new(),
        banners: sill_surfaces::surfaces::banners::BannerLink::new(),
        corners: sill_surfaces::surfaces::hot_corners::CornerLink::new(),
        switcher: sill_surfaces::surfaces::switcher::SwitcherLink::new(),
        lock: sill_surfaces::surfaces::lock::LockLink::new(),
        edit_widgets: sill_surfaces::surfaces::edit_widgets::EditWidgetsLink::new(),
        widgets_writer: crate::edit_widgets::writer(
            settings_dir.clone(),
            sill_surfaces::surfaces::desktop_widgets::PlacementFile::in_dirs(&env.dirs),
            settings_tx.clone(),
        ),
        power_menu_keys: args.power_menu_keys(),
    });
    // The notifications service's policy from the settings and the focus mode; the first one
    // sent ends its passive start.
    if let Some(links) = crate::links::get() {
        follow(
            runtime.handle(),
            settings_feed.0.clone(),
            crate::notifications::Bridge {
                focus: links.focus.mode(),
            },
            services.notifications.control.clone(),
        );
    }

    // The lock and the polkit agent are sill's own; both services are passive until told.
    let _ = services.locker.control.send(LockerCommand::Configure);
    let _ = services
        .polkit_agent
        .control
        .send(PolkitCommand::Policy(AgentPolicy::Own));
    // sill's own idle follows `idle.*` and `session.lock_grace_s`; the service is passive until
    // this first policy, and `Off` anywhere but a sill session with no cosmic-idle.
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::idle::Bridge {
            times: env.idle_times,
        },
        services.idle.control.clone(),
    );
    // The launcher v2 providers and the clipboard history follow `launcher.*`; both are passive
    // until this first policy.
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::launcher_policy::SearchBridge,
        services.results.control.clone(),
    );
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::launcher_policy::ClipboardBridge,
        services.clipboard.control.clone(),
    );
    // The displays follow `display.*`; the service is inert until this first policy.
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::displays::Bridge,
        services.displays.control.clone(),
    );

    let config = HostConfig {
        first_mouse_window_ms: shell_host::config::Ms(u32::from(
            initial_settings.menus.first_mouse_window_ms.0,
        )),
        ..sill_surfaces::host_config()
    };
    // The calendar's sources and the screenshots' folders and tool, from the settings (both
    // services passive until then).
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::calendar::Bridge {
            home: env.dirs.home.clone(),
        },
        services.events.control.clone(),
    );
    follow(
        runtime.handle(),
        settings_feed.0.clone(),
        crate::screenshots::Bridge {
            home: env.dirs.home.clone(),
        },
        services.screenshots.control.clone(),
    );

    let host = match Host::<ShellMsg>::connect(config) {
        Ok(host) => host,
        Err(error) => return fatal(&format!("connecting to the compositor: {error}")),
    };
    let handle = host.handle();

    // Detached: it accepts connections until the process exits with the host (doc on
    // `ipc_server::spawn`).
    let _ipc_thread = server::spawn(door, handle.clone());
    // The own lock screen's wish to hold the session lock.
    crate::lock::spawn_follower(runtime.handle(), &services, handle.clone());
    // The idle dim surfaces are mapped while the idle service dims.
    crate::idle_dim::spawn_follower(runtime.handle(), &services, handle.clone());

    if let Some(dir) = settings_dir.clone() {
        // `sill_settings::watch` calls `tokio::spawn` in its own body, which needs an entered
        // runtime on the calling thread; `host.run` does not enter it until later.
        let _entered = runtime.enter();
        match sill_settings::watch(&dir) {
            Ok(mut watch) => {
                let handle = handle.clone();
                runtime.spawn(async move {
                    while let Some(file) = watch.changed().await {
                        if handle.send(ShellMsg::Settings(Box::new(file))).is_err() {
                            break;
                        }
                    }
                });
            }
            Err(error) => eprintln!("sill: watching {}: {error}", dir.display()),
        }
    }

    let mut outputs = Outputs::default();
    let launcher = LauncherLink::new();
    let osd = OsdLink::new();
    // The hot corners' firings, carried out off the host thread.
    if let Some(links) = crate::links::get() {
        crate::corners::spawn_hits(
            runtime.handle(),
            &links.corners,
            settings_feed.0.clone(),
            services.clone(),
            launcher.clone(),
        );
    }
    let mut shell_lock = crate::lock::ShellLock::default();
    let result = host.run(move |ctx, event| {
        run_event(
            ctx,
            &mut outputs,
            &mut shell_lock,
            &services,
            &settings_feed,
            settings_dir.as_deref(),
            &settings_tx,
            &launcher,
            &osd,
            event,
        );
    });

    shutdown_runtime(runtime);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fatal(&format!("{error}")),
    }
}

/// The bound [`shutdown_runtime`] gives every service's blocking-pool thread to finish once
/// `host.run` has returned (`ctx.exit()`; every surface, and with it every clone of `Services`,
/// is already gone by then — shell-host's own `Host::run` clears every surface before
/// returning). Comfortably above `sill-services`' own bounded waits (its `cosmic-randr` call is
/// 2 s), but not unbounded.
const RUNTIME_SHUTDOWN_BOUND: Duration = Duration::from_secs(4);

/// Ends `runtime`, waiting at most [`RUNTIME_SHUTDOWN_BOUND`] for every spawned task's thread to
/// finish. Dropping a `tokio::runtime::Runtime` plainly waits forever for that, and a
/// `spawn_blocking` task that calls into something that hangs (`cosmic-randr` under a
/// compositor without `zwlr_output_manager_v1`) would otherwise keep
/// the daemon from ever exiting after `sill quit` already got its answer. Past the bound, that
/// thread (and whatever it is stuck in) is left running and the process exits without it; a
/// warning on stderr says so.
fn shutdown_runtime(runtime: tokio::runtime::Runtime) {
    shutdown_runtime_within(runtime, RUNTIME_SHUTDOWN_BOUND);
}

/// [`shutdown_runtime`]'s body, `bound` taken as a parameter so a test can shrink it rather than
/// waiting out the real one.
fn shutdown_runtime_within(runtime: tokio::runtime::Runtime, bound: Duration) {
    let started = Instant::now();
    runtime.shutdown_timeout(bound);
    if started.elapsed() >= bound {
        eprintln!(
            "sill: the async runtime did not shut down within {bound:?}; a service's thread is \
             still running and is left behind as the process exits"
        );
    }
}

fn fatal(message: &str) -> ExitCode {
    eprintln!("sill: {message}");
    ExitCode::FAILURE
}

/// Every event, dispatched: an IPC request (needs `services` and the settings), or everything
/// else via `on_event` followed by spawning whatever it left missing. The launcher panel's
/// keyboard focus goes to `launcher` (a focus loss closes it, design/06 §20.2); `osd` is what
/// every output's indicator follows.
#[allow(clippy::too_many_arguments)]
fn run_event(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    shell_lock: &mut crate::lock::ShellLock,
    services: &Services,
    settings_feed: &SettingsFeed,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    launcher: &LauncherLink,
    osd: &OsdLink,
    event: HostEvent<ShellMsg>,
) {
    match event {
        HostEvent::App(ShellMsg::Ipc { request, reply }) => {
            // `Toggle` is timed from here (the IPC request's arrival) regardless of what
            // `route` does with it: the `launcher.open_latency_ms` line (module doc).
            // Placed at the request's arrival rather than inside the launcher's dispatch so that
            // function's body can change freely — spawning the panel for real, keeping it warm,
            // whatever the surface needs — without moving this: the elapsed time this measures
            // grows on its own, from "IPC overhead only" today to "IPC arrival to the frame that
            // presents the shown panel" once `route` itself waits on that frame (e.g.
            // via `HostCtx::take_stats` after spawning/showing the surface).
            let toggled_at = matches!(
                request,
                Request::Show(ShowRequest::Launcher(LauncherRequest::Toggle))
            )
            .then(Instant::now);
            let reach = Reach {
                services,
                outputs,
                links: crate::links::get(),
                launcher,
                osd,
                settings_dir,
                settings: settings_tx,
            };
            let response = route(ctx, &reach, request);
            if let Some(toggled_at) = toggled_at {
                latency::log_open_latency(toggled_at.elapsed());
            }
            let _ = reply.send(response);
        }
        HostEvent::App(ShellMsg::Lock(wish)) => {
            if let Some(links) = crate::links::get() {
                crate::lock::apply(
                    ctx,
                    shell_lock,
                    wish,
                    || lock_factory(services, settings_feed, &links.lock),
                    &links.lock,
                    outputs,
                    services,
                );
            }
        }
        HostEvent::App(ShellMsg::Dim(dim)) => crate::idle_dim::apply(ctx, outputs, dim),
        other => {
            if let Some(links) = crate::links::get() {
                crate::lock::on_host_event(ctx, shell_lock, &other, &links.lock, services);
            }
            if let HostEvent::KeyboardFocus { id, focus } = &other
                && outputs.outputs.iter().any(|(_, s)| s.launcher == Some(*id))
            {
                launcher.focus(*focus);
            }
            if let HostEvent::KeyboardFocus { id, focus } = &other
                && outputs
                    .outputs
                    .iter()
                    .any(|(_, s)| s.power_menu == Some(*id))
                && let Some(links) = crate::links::get()
            {
                links.power_menu.focus(*focus);
            }
            // The engine has no config until told one; the watcher's and the reload's changes
            // land through `HostEvent::App(ShellMsg::Settings(_))` and `Request::Reload` below.
            if matches!(other, HostEvent::Started) {
                ctx.set_scroll_config(crate::scroll::config(&settings_tx.borrow()));
            }
            // The watch's re-read reaches `settings_tx` (every `SettingsFeed` follower: the
            // policies, `crate::policy::follow`, the locker, corners, the focus mode) the same
            // way `dispatch_reload` does for `sill reload`; geometry follows the same event just
            // below.
            settings::publish_watched(settings_tx, &other);
            crate::displays::on_host_event(services, &other);
            on_event(ctx, outputs, other);
            spawn_missing_bars(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_wallpapers(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_docks(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_launchers(ctx, outputs, services, settings_feed, settings_tx, launcher);
            crate::power_menu::spawn_missing(ctx, outputs, |root| {
                surface_root(root, services, settings_feed)
            });
            // The polkit prompt over the power menu, and the switcher (both Overlay).
            crate::polkit::spawn_missing(ctx, outputs, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::switcher::spawn_missing(ctx, outputs, |root| {
                surface_root(root, services, settings_feed)
            });
            // Before the OSD, so the OSD draws over a banner (both on the Overlay layer).
            crate::notifications::spawn_missing(ctx, outputs, settings_tx, |root| {
                surface_root(root, services, settings_feed)
            });
            // The screenshot thumbnail after the banners and before the OSD (all Overlay); the
            // desktop widgets (Bottom) and the hot corners (Overlay, never painted).
            crate::screenshots::spawn_missing(ctx, outputs, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::widgets::spawn_missing(ctx, outputs, settings_tx, &services.dirs, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::edit_widgets::spawn_missing(ctx, outputs, &services.dirs, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::corners::spawn_missing(ctx, outputs, settings_tx, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::osd::spawn_missing(ctx, outputs, settings_tx, osd, |root| {
                surface_root(root, services, settings_feed)
            });
            // The idle dim last, over every other Overlay surface; the session-lock
            // surfaces are above every layer surface anyway.
            let dim = services.idle.state.borrow().dim;
            crate::idle_dim::spawn_missing(ctx, outputs, dim, |root| {
                surface_root(root, services, settings_feed)
            });
            if let Some(links) = crate::links::get() {
                links
                    .banners
                    .set(crate::notifications::banner_output(outputs));
            }
        }
    }
}

/// The host event handler: outputs come and go, surfaces closed by the compositor are noted for
/// respawning, and a settled settings change resizes the bar. IPC requests are answered by
/// [`run_event`] (frozen `on_event` has no way to reach `Services`, which some of them need);
/// see this file's module doc.
pub fn on_event(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs, event: HostEvent<ShellMsg>) {
    match event {
        HostEvent::Started
        | HostEvent::OutputAdded(_)
        | HostEvent::OutputChanged(_)
        | HostEvent::OutputRemoved(_) => reconcile_outputs(ctx, outputs),
        HostEvent::SurfaceClosed { id, reason } => on_surface_closed(outputs, id, reason),
        // Answered by `run_event` (`crate::lock::on_host_event`), which holds the lock.
        HostEvent::KeyboardFocus { .. } | HostEvent::Lock { .. } => {}
        HostEvent::App(ShellMsg::Settings(file)) => {
            resize_bars(ctx, outputs, &file);
            ctx.set_scroll_config(crate::scroll::config(&file));
        }
        // Answered by `run_event`, which holds the lock's state.
        HostEvent::App(ShellMsg::Lock(_)) => {}
        // Answered by `run_event`, which maps the idle dim surfaces.
        HostEvent::App(ShellMsg::Dim(_)) => {}
        HostEvent::App(ShellMsg::Ipc { reply, .. }) => {
            // Never reached through `run_event`, which answers `Ipc` itself; answered rather
            // than dropped in case another caller ever delivers one here directly.
            let _ = reply.send(Response::Failed(Failure {
                kind: FailureKind::Internal,
                message: "ipc requests are dispatched before on_event".to_owned(),
            }));
        }
    }
}

/// `ctx.outputs()` against `outputs`, applying every action `reconcile` names: a new output's
/// row is added empty (marking it for [`spawn_missing_bars`]), a changed output's row is kept
/// with its existing surfaces, a removed output's surfaces are closed.
fn reconcile_outputs(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs) {
    let now = ctx.outputs();
    for action in reconcile(outputs, &now) {
        match action {
            OutputAction::SpawnAll(info) => outputs.set(info, OutputSurfaces::default()),
            OutputAction::Resize(info) => {
                let surfaces = outputs.get(info.id).cloned().unwrap_or_default();
                outputs.set(info, surfaces);
            }
            OutputAction::CloseAll(id) => {
                if let Some(surfaces) = outputs.remove(id) {
                    close_every(ctx, &surfaces);
                }
            }
        }
    }
}

fn close_every(ctx: &mut HostCtx<ShellMsg>, surfaces: &OutputSurfaces) {
    for id in surfaces.ids() {
        ctx.close(id);
    }
}

/// A surface the compositor (or a failed spawn) closed: forget it. An unrequested close leaves
/// its output's row with that field `None`, which `spawn_missing_bars` treats as "needs one".
fn on_surface_closed(outputs: &mut Outputs, id: SurfaceId, reason: CloseReason) {
    let Some((info, surfaces)) = outputs.row_with(id) else {
        return;
    };
    surfaces.clear(id);
    if reason != CloseReason::Requested {
        eprintln!(
            "sill: the bar on {:?} closed unexpectedly ({reason:?}); respawning",
            info.name
        );
    }
}

/// A settled `settings.toml`: re-apply every bar's geometry (only `bar.height_px` can change
/// it; the rest is drawn by the bar's own component).
fn resize_bars(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs, file: &ShellFile) {
    for (info, surfaces) in &outputs.outputs {
        let Some(id) = surfaces.bar else { continue };
        let config = sill_surfaces::surfaces::bar::bar_layer(info, &file.bar);
        ctx.apply(id, shell_host::SurfaceOp::SetSize(config.size));
        ctx.apply(
            id,
            shell_host::SurfaceOp::SetExclusiveZone(config.exclusive),
        );
    }
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
fn lock_factory(
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

#[cfg(test)]
mod shutdown_tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::shutdown_runtime_within;

    /// A `spawn_blocking` task that never returns — the shape of `cosmic-randr` hanging under a
    /// compositor without `zwlr_output_manager_v1` — does not keep
    /// [`shutdown_runtime_within`] waiting past its bound; the leaked thread is left running
    /// (past the bound `forever` is never drained again, so it stays parked) and the call still
    /// returns.
    #[test]
    fn a_stuck_blocking_task_does_not_hang_the_bound() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("{e}"));
        let (never_sent, forever) = mpsc::channel::<()>();
        runtime.spawn_blocking(move || {
            // `never_sent` moves in and is held here (not dropped) for as long as this closure
            // runs, so `forever.recv()` below blocks rather than seeing a disconnected sender.
            let _held = never_sent;
            let _ = forever.recv();
        });
        let bound = Duration::from_millis(200);
        let started = Instant::now();
        shutdown_runtime_within(runtime, bound);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "shutdown waited well past its {bound:?} bound"
        );
    }

    /// A runtime with nothing left running shuts down well inside its bound: the bound is a
    /// ceiling, not a fixed wait.
    #[test]
    fn a_clean_runtime_shuts_down_promptly() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("{e}"));
        let bound = Duration::from_secs(2);
        let started = Instant::now();
        shutdown_runtime_within(runtime, bound);
        assert!(
            started.elapsed() < bound,
            "a clean shutdown should not need the whole bound"
        );
    }
}
