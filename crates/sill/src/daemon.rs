//! `sill run`: take the single-instance lock, start the services and the IPC server, connect
//! the host, and keep one set of surfaces per output (plan: "daemon uses
//! `latchkey::Agent::new(\"sill\").listen()` (AlreadyRunning -> exit 0)").
//!
//! [`on_event`] is the frozen event handler; it owns the parts that need only `HostCtx` and
//! [`Outputs`] (output bookkeeping via [`crate::outputs::reconcile`], closing a surface the
//! compositor took away, resizing the bar when settled settings say to). Spawning a surface
//! needs [`sill_services::Services`] and the live settings, neither of which fits `on_event`'s
//! signature, so [`run`] wraps it: [`run_event`] answers IPC requests (which also need
//! `Services`) and, after delegating every other event to `on_event`, spawns whatever `on_event`
//! marked as missing (an `OutputSurfaces` row with `bar: None`).

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use shell_host::scale::{Size as BufferSize, buffer_size};
use shell_host::{CloseReason, Host, HostConfig, HostCtx, HostEvent, OutputInfo, Root, SurfaceId};
use sill_ipc::{
    DebugRequest, DockRequest, Failure, FailureKind, LauncherRequest, Request, Response,
    SurfaceName,
};
use sill_launcher::AppId;
use sill_services::{OutputName, PinAt, PinEvent, Services, WallpaperCommand};
use sill_settings::{SettingsFeed, ShellFile, WallpaperDrawer};
use sill_surfaces::surfaces::launcher::LauncherLink;
use sill_surfaces::surfaces::osd::OsdLink;
use tokio::sync::watch;

use crate::ipc_server;
use crate::msg::ShellMsg;
use crate::outputs::{
    OutputAction, OutputSurfaces, Outputs, bar_namespace, reconcile, wallpaper_namespace,
};

/// Run the daemon until `quit` or a fatal host error.
pub fn run() -> ExitCode {
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

    let services = sill_services::Services::spawn(runtime.handle());

    let settings_dir = sill_settings::settings_dir();
    let initial_settings = settings_dir
        .as_deref()
        .map(sill_settings::load)
        .unwrap_or_default();
    let (settings_tx, settings_rx) = watch::channel(initial_settings.clone());
    let settings_feed = SettingsFeed(settings_rx);
    crate::displays::send_policy(&services, &initial_settings);
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
    });
    // M6: the notifications service's policy from the settings and the focus mode; the first
    // one sent ends its passive start (FINDINGS "M6 freeze").
    if let Some(links) = crate::links::get() {
        crate::notifications::spawn_policy(
            runtime.handle(),
            settings_feed.0.clone(),
            links.focus.mode(),
            services.notifications_control.clone(),
        );
    }

    // M7: the locker follows `session.locker` (FINDINGS "M7").
    crate::session::spawn_locker_policy(
        runtime.handle(),
        settings_feed.0.clone(),
        services.locker_control.clone(),
    );
    // M11: the own polkit agent registers only while `session.polkit_agent` is `sill`; the
    // service is passive until this first policy (FINDINGS "M11 freeze").
    crate::polkit::spawn_policy(
        runtime.handle(),
        settings_feed.0.clone(),
        services.polkit_control.clone(),
    );

    let config = HostConfig {
        first_mouse_window_ms: shell_host::config::Ms(u32::from(
            initial_settings.menus.first_mouse_window_ms.0,
        )),
        prepaint: sill_surfaces::prepaint(),
        ..HostConfig::default()
    };
    // M10: the calendar's sources and the screenshots' folders and tool, from the settings
    // (both services passive until then; lanes a and c fill the followers).
    crate::calendar::spawn_policy(
        runtime.handle(),
        settings_feed.0.clone(),
        services.events_control.clone(),
    );
    crate::screenshots::spawn_policy(
        runtime.handle(),
        settings_feed.0.clone(),
        services.shots_control.clone(),
    );

    let host = match Host::<ShellMsg>::connect(config) {
        Ok(host) => host,
        Err(error) => return fatal(&format!("connecting to the compositor: {error}")),
    };
    let handle = host.handle();

    // Detached: it accepts connections until the process exits with the host (doc on
    // `ipc_server::spawn`).
    let _ipc_thread = ipc_server::spawn(door, handle.clone());
    // M11: the own lock screen's wish to hold the session lock (lane a fills the follower).
    crate::lock::spawn_follower(runtime.handle(), &services, handle.clone());

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
    // M10: the hot corners' firings, carried out off the host thread (lane d fills it).
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
/// 2 s: FINDINGS G51), but not unbounded.
const RUNTIME_SHUTDOWN_BOUND: Duration = Duration::from_secs(4);

/// Ends `runtime`, waiting at most [`RUNTIME_SHUTDOWN_BOUND`] for every spawned task's thread to
/// finish. Dropping a `tokio::runtime::Runtime` plainly waits forever for that, and a
/// `spawn_blocking` task that calls into something that hangs (`cosmic-randr` under a
/// compositor without `zwlr_output_manager_v1`, e.g. KWin: FINDINGS G51) would otherwise keep
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
            // `dispatch_request` does with it: M4 perf-ime's `launcher.open_latency_ms` line
            // (module doc; FINDINGS.md "M4 perf-ime"). Placed at the request's arrival rather
            // than inside `dispatch_launcher` so M4 launcher-ui can freely rewrite that
            // function's body — spawning the panel for real, keeping it warm, whatever the
            // surface needs — without colliding with this edit: the elapsed time this measures
            // grows on its own, from "IPC overhead only" today to "IPC arrival to the frame
            // that presents the shown panel" once `dispatch_request` itself waits on that frame
            // (e.g. via `HostCtx::take_stats` after spawning/showing the surface).
            let toggled_at =
                matches!(request, Request::Launcher(LauncherRequest::Toggle)).then(Instant::now);
            let response = dispatch_request(
                ctx,
                outputs,
                services,
                settings_dir,
                settings_tx,
                launcher,
                osd,
                request,
            );
            if let Some(toggled_at) = toggled_at {
                log_open_latency(toggled_at.elapsed());
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
            crate::displays::on_host_event(services, &other);
            on_event(ctx, outputs, other);
            spawn_missing_bars(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_wallpapers(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_docks(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_launchers(ctx, outputs, services, settings_feed, settings_tx, launcher);
            crate::power_menu::spawn_missing(ctx, outputs, &services.session, |root| {
                surface_root(root, services, settings_feed)
            });
            // M11: the polkit prompt over the power menu, and the switcher (both Overlay).
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
            // M10: the screenshot thumbnail after the banners and before the OSD (all Overlay);
            // the desktop widgets (Bottom) and the hot corners (Overlay, never painted).
            crate::screenshots::spawn_missing(ctx, outputs, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::widgets::spawn_missing(ctx, outputs, settings_tx, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::corners::spawn_missing(ctx, outputs, settings_tx, |root| {
                surface_root(root, services, settings_feed)
            });
            crate::osd::spawn_missing(ctx, outputs, settings_tx, osd, |root| {
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
        // M11: answered by `run_event` (`crate::lock::on_host_event`), which holds the lock.
        HostEvent::KeyboardFocus { .. } | HostEvent::Lock { .. } => {}
        HostEvent::App(ShellMsg::Settings(file)) => {
            resize_bars(ctx, outputs, &file);
            ctx.set_scroll_config(crate::scroll::config(&file));
        }
        // Answered by `run_event`, which holds the lock's state.
        HostEvent::App(ShellMsg::Lock(_)) => {}
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
/// `Prepare` (there is no `sill_services::OutputName` to key it by), the same way the bar's own
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
        let prepared = services.wallpaper_control.send(WallpaperCommand::Prepare {
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

/// Spawn the launcher's two surfaces for every output whose row has none yet, both hidden at
/// once: they stay alive and are shown and hidden with `set_visibility`, following `launcher`
/// (plan "Launcher v1"). The catcher is spawned first.
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

/// `root` with the contexts every surface the daemon spawns is given: the services, the
/// settings, quire's rect measurer, `ds_native::measure::MEASURE` (re-exported as
/// `sill_surfaces::MEASURE`), and its focus write, `ds_native::focus::FOCUS` (re-exported as
/// `sill_surfaces::FOCUS`), which are what `ds_native::measure::provide()` and
/// `ds_native::focus::provide()` provide from inside a root component. Without them a rect read
/// or a focus waits a frame and the panic hook prints the collision (FINDINGS Q10, F81,
/// "Adoption of the bar gaps", "Adoption of the launcher gaps"). Every spawn goes through here,
/// so a new surface cannot forget them.
pub(crate) fn surface_root(root: Root, services: &Services, settings_feed: &SettingsFeed) -> Root {
    let root = root
        .context(services.clone())
        .context(settings_feed.clone())
        .context(sill_surfaces::MEASURE)
        .context(sill_surfaces::FOCUS);
    // M5: the focus mode, the power menu and the control center's asks; M6: the notification
    // center and the banners' output; M10: the hot corners' firings; M11: the switcher's asks
    // and the lock prompt's output (`crate::links`).
    match crate::links::get() {
        Some(links) => root
            .context(links.focus.clone())
            .context(links.power_menu.clone())
            .context(links.control_center.clone())
            .context(links.notification_center.clone())
            .context(links.banners.clone())
            .context(links.corners.clone())
            .context(links.switcher.clone())
            .context(links.lock.clone()),
        None => root,
    }
}

/// Each output's lock surface for shell-host's session lock (M11): `LockScreen` with the
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

/// Answer one IPC request. A display key's change, and a made-up one, show on the OSD.
#[allow(clippy::too_many_arguments)]
fn dispatch_request(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &Outputs,
    services: &Services,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    launcher: &LauncherLink,
    osd: &OsdLink,
    request: Request,
) -> Response {
    match request {
        Request::Ping => Response::Pong,
        Request::Launcher(request) => dispatch_launcher(outputs, launcher, request),
        Request::Dock(request) => dispatch_dock(services, request),
        Request::Reload => {
            let response = dispatch_reload(settings_dir, settings_tx);
            crate::displays::send_policy(services, &settings_tx.borrow());
            ctx.set_scroll_config(crate::scroll::config(&settings_tx.borrow()));
            response
        }
        Request::Quit => {
            ctx.exit();
            Response::Done
        }
        Request::Debug(DebugRequest::Osd(fake)) => {
            crate::osd::fake(osd, services, outputs, &settings_tx.borrow(), fake)
        }
        Request::Debug(DebugRequest::Notify(fake)) => crate::notifications::fake(services, fake),
        Request::Debug(DebugRequest::Screenshot(fake)) => crate::screenshots::fake(services, fake),
        Request::Debug(DebugRequest::Corner(fake)) => {
            crate::corners::fake(outputs, crate::links::get(), fake)
        }
        Request::Debug(DebugRequest::Auth(fake)) => crate::lock::fake_auth(services, fake),
        Request::Debug(DebugRequest::Polkit(fake)) => crate::polkit::fake(services, fake),
        Request::Debug(DebugRequest::Switcher(fake)) => {
            crate::switcher::fake(outputs, crate::links::get(), fake)
        }
        Request::Debug(request) => dispatch_debug(ctx, outputs, request),
        Request::Display(request) => {
            let response = crate::displays::dispatch(services, request);
            let file = settings_tx.borrow();
            crate::osd::after_display(osd, services, outputs, &file, request, &response);
            response
        }
        Request::PowerMenu(request) => {
            crate::power_menu::dispatch_power_menu(outputs, crate::links::get(), request)
        }
        Request::ControlCenter(request) => {
            crate::power_menu::dispatch_control_center(outputs, crate::links::get(), request)
        }
        Request::Notifications(request) => {
            crate::notifications::dispatch(outputs, crate::links::get(), services, request)
        }
        Request::Lock => crate::session::lock(services),
        Request::Screenshot(request) => crate::screenshots::dispatch(services, request),
        Request::Widgets(request) => crate::widgets::dispatch(settings_dir, settings_tx, request),
        Request::Switcher(request) => {
            crate::switcher::dispatch(outputs, crate::links::get(), request)
        }
        Request::LockStatus(_) => crate::lock::status(services),
        Request::Polkit(request) => crate::polkit::dispatch(services, request),
    }
}

/// `sill launcher toggle|open|close`: the shared launcher state, which both launcher surfaces
/// follow. It opens on the first output with a launcher (the output under the pointer is not
/// known to the daemon; FINDINGS F140).
fn dispatch_launcher(
    outputs: &Outputs,
    launcher: &LauncherLink,
    request: LauncherRequest,
) -> Response {
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.launcher.is_some())
        .map(|(info, _)| info.id);
    match (request, target) {
        (LauncherRequest::Close, _) => launcher.close(),
        (LauncherRequest::Toggle, Some(output)) => launcher.toggle(output),
        (LauncherRequest::Open { query }, Some(output)) => launcher.open(output, query),
        (LauncherRequest::Toggle | LauncherRequest::Open { .. }, None) => {
            return unsupported("no output has a launcher surface");
        }
    }
    Response::Done
}

fn dispatch_dock(services: &Services, request: DockRequest) -> Response {
    let event = match request {
        DockRequest::Pin { app } => PinEvent::Pin {
            app: AppId(app.0),
            at: PinAt::End,
        },
        DockRequest::Unpin { app } => PinEvent::Unpin(AppId(app.0)),
    };
    match services.pins_control.send(event) {
        Ok(()) => Response::Done,
        Err(error) => internal(&error),
    }
}

/// Re-read `settings.toml` and publish it (the bar and every future surface pick it up through
/// [`SettingsFeed`] and, for geometry, through `ShellMsg::Settings`). Dock pins are not re-read
/// here: `sill_services::pins::load` is still `todo!()` (M2 launch); FINDINGS.md notes the gap.
fn dispatch_reload(dir: Option<&Path>, settings_tx: &watch::Sender<ShellFile>) -> Response {
    let Some(dir) = dir else {
        return Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: "no settings directory (XDG_CONFIG_HOME and HOME are both unset)".to_owned(),
        });
    };
    settings_tx.send_replace(sill_settings::load(dir));
    Response::Done
}

fn dispatch_debug(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &Outputs,
    request: DebugRequest,
) -> Response {
    match request {
        DebugRequest::Stats => Response::Stats(collect_stats(ctx, outputs)),
        DebugRequest::Pointer { surface, at } => act(ctx, surface, DebugAction::Pointer(at)),
        DebugRequest::Leave { surface } => act(ctx, surface, DebugAction::Leave),
        DebugRequest::Capture { surface, path } => act(ctx, surface, DebugAction::Capture(path)),
        DebugRequest::Click {
            surface,
            at,
            button,
        } => act(ctx, surface, DebugAction::Click(at, button)),
        // Answered by `dispatch_request`, which holds the OSD's link.
        DebugRequest::Osd(_) => unsupported("the OSD's made-up change is answered elsewhere"),
        DebugRequest::Notify(_) => unsupported("a made-up notification is answered elsewhere"),
        DebugRequest::Screenshot(_) => unsupported("a made-up screenshot is answered elsewhere"),
        DebugRequest::Corner(_) => unsupported("a made-up corner hit is answered elsewhere"),
        DebugRequest::Auth(_) => unsupported("a made-up verdict is answered elsewhere"),
        DebugRequest::Polkit(_) => unsupported("a made-up polkit request is answered elsewhere"),
        DebugRequest::Switcher(_) => unsupported("a made-up switcher is answered elsewhere"),
    }
}

/// What [`act`] was asked to do, independent of `shell_host::Inject` (only defined under
/// shell-host's `debug-inject` feature) so `dispatch_debug` and `act` compile either way.
enum DebugAction {
    Pointer(sill_ipc::Point),
    Leave,
    Capture(std::path::PathBuf),
    Click(sill_ipc::Point, sill_ipc::PointerButton),
}

/// Find `surface` by shell-host's own naming (`HostCtx::surface_named`, unconditional: it needs
/// neither `debug` nor shell-host's `debug-inject`) and deliver `action` to it.
fn act(ctx: &mut HostCtx<ShellMsg>, surface: SurfaceName, action: DebugAction) -> Response {
    let Some(id) = ctx.surface_named(&shell_host::SurfaceName(surface.0.clone())) else {
        return unknown_surface(&surface);
    };
    apply_injection(ctx, id, action)
}

/// Pointer/leave/capture, through shell-host's typed `HostCtx::inject` (feature `debug`; a
/// capture's success and size come back directly through `Injected`, no bridging over stdout —
/// FINDINGS.md G1 resolved upstream).
#[cfg(feature = "debug")]
fn apply_injection(ctx: &mut HostCtx<ShellMsg>, id: SurfaceId, action: DebugAction) -> Response {
    let inject = match to_inject(action) {
        Ok(inject) => inject,
        Err(refusal) => return refusal,
    };
    match ctx.inject(id, inject) {
        Ok(_injected) => Response::Done,
        Err(error) => internal(&error),
    }
}

/// `action` as shell-host's injection; every pointer button maps through (G10 is closed).
#[cfg(feature = "debug")]
fn to_inject(action: DebugAction) -> Result<shell_host::Inject, Response> {
    let point = |at: sill_ipc::Point| shell_host::Point { x: at.x, y: at.y };
    Ok(match action {
        DebugAction::Pointer(at) => shell_host::Inject::Pointer { at: point(at) },
        DebugAction::Leave => shell_host::Inject::Leave,
        DebugAction::Capture(path) => shell_host::Inject::Capture { path },
        DebugAction::Click(at, button) => shell_host::Inject::Click {
            at: point(at),
            button: to_host_button(button),
        },
    })
}

#[cfg(feature = "debug")]
fn to_host_button(button: sill_ipc::PointerButton) -> shell_host::debug::PointerButton {
    match button {
        sill_ipc::PointerButton::Primary => shell_host::debug::PointerButton::Primary,
        sill_ipc::PointerButton::Secondary => shell_host::debug::PointerButton::Secondary,
        sill_ipc::PointerButton::Middle => shell_host::debug::PointerButton::Middle,
    }
}

#[cfg(not(feature = "debug"))]
fn apply_injection(_ctx: &mut HostCtx<ShellMsg>, _id: SurfaceId, action: DebugAction) -> Response {
    // Naming the action keeps every variant's payload read in this build too.
    let what = match action {
        DebugAction::Pointer(at) => format!("pointer at {},{}", at.x, at.y),
        DebugAction::Click(at, button) => format!("{button:?} click at {},{}", at.x, at.y),
        DebugAction::Leave => "leave".to_owned(),
        DebugAction::Capture(path) => format!("capture to {}", path.display()),
    };
    unsupported(&format!("built without the debug feature ({what})"))
}

/// The dock's layer namespace on `info` (`sill-dock-<output>`), which `debug stats` reports.
fn dock_namespace(info: &OutputInfo) -> String {
    sill_surfaces::surfaces::dock::dock_layer(info, &sill_settings::DockSettings::default())
        .namespace
}

/// The launcher panel's and catcher's namespaces on `info`, which `debug stats` reports.
fn launcher_namespaces(info: &OutputInfo) -> (String, String) {
    use sill_surfaces::surfaces::launcher::{catcher_layer, panel_layer};
    let panel = panel_layer(info, &Default::default(), &Default::default()).namespace;
    (panel, catcher_layer(info).namespace)
}

/// Every known bar's, wallpaper's, dock's, launcher surface's and OSD's frame statistics, through the
/// always-public `HostCtx::take_stats` (needs neither the `debug` feature nor shell-host's
/// `debug-inject`; draining, so each report covers only the time since the previous one).
fn collect_stats(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<sill_ipc::SurfaceStats> {
    let ids: Vec<(String, SurfaceId)> = outputs
        .outputs
        .iter()
        .flat_map(|(info, surfaces)| {
            let bar = surfaces.bar.map(|id| (bar_namespace(info), id));
            let wallpaper = surfaces.wallpaper.map(|id| (wallpaper_namespace(info), id));
            let dock = surfaces.dock.map(|id| (dock_namespace(info), id));
            let (panel, catcher) = launcher_namespaces(info);
            let launcher = surfaces.launcher.map(|id| (panel, id));
            let catcher = surfaces.catcher.map(|id| (catcher, id));
            let osd = surfaces.osd.map(|id| (crate::osd::osd_namespace(info), id));
            let power_menu = surfaces.power_menu.map(|id| {
                let layer = sill_surfaces::surfaces::power_menu::power_menu_layer(info);
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
            let polkit = surfaces
                .polkit_prompt
                .map(|id| (crate::polkit::namespace(info), id));
            let switcher = surfaces
                .switcher
                .map(|id| (crate::switcher::namespace(info), id));
            let corners: Vec<_> = crate::corners::namespaces(info)
                .into_iter()
                .filter_map(|(corner, name)| surfaces.corners.get(corner).map(|id| (name, id)))
                .collect();
            bar.into_iter()
                .chain(wallpaper)
                .chain(dock)
                .chain(launcher)
                .chain(catcher)
                .chain(osd)
                .chain(power_menu)
                .chain(banners)
                .chain(center)
                .chain(thumbnail)
                .chain(desktop)
                .chain(corners)
                .chain(polkit)
                .chain(switcher)
        })
        .collect();
    // The lock surfaces are the host's (shell-host names each `sill-lock/<output>`), not a row
    // of `outputs`, so they are found by name while a lock is up (G191).
    let locks: Vec<(String, SurfaceId)> = outputs
        .outputs
        .iter()
        .filter_map(|(info, _)| info.name.clone())
        .filter_map(|name| {
            let label = format!("{}/{name}", crate::lock::host::LOCK_NAMESPACE);
            let id = ctx.surface_named(&shell_host::SurfaceName(label.clone()))?;
            Some((label, id))
        })
        .collect();
    ids.into_iter()
        .chain(locks)
        .filter_map(|(name, id)| Some(to_ipc_stats(name, ctx.take_stats(id)?)))
        .collect()
}

fn to_ipc_stats(name: String, stats: shell_host::FrameStats) -> sill_ipc::SurfaceStats {
    sill_ipc::SurfaceStats {
        surface: SurfaceName(name),
        frames: stats.counters.frames,
        callbacks: stats.counters.callbacks,
        commits: stats.counters.commits,
        skipped: stats.counters.skipped,
        watchdog_fired: stats.counters.watchdog_fired,
        work: to_ipc_micros(stats.work),
        interval: to_ipc_micros(stats.interval),
        samples: stats.samples,
        vm_rss_kib: stats.vm_rss_kib,
    }
}

fn to_ipc_micros(p: shell_host::Percentiles) -> sill_ipc::Micros {
    sill_ipc::Micros {
        p50_us: p.p50_us,
        p95_us: p.p95_us,
        max_us: p.max_us,
    }
}

/// Prints [`open_latency_line`] on stderr: the M4 perf-ime latency line `dev/accept-launcher.sh`
/// parses for its p50/p95/max over 20 toggles.
fn log_open_latency(elapsed: Duration) {
    eprintln!("{}", open_latency_line(elapsed));
}

/// The stderr line for one `sill launcher toggle`'s latency, `elapsed` since the IPC request
/// arrived. Its own function (rather than inlined in [`log_open_latency`]) so
/// `crates/sill/tests/latency.rs` can check the arithmetic without a wall clock.
pub fn open_latency_line(elapsed: Duration) -> String {
    format!(
        "launcher.open_latency_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    )
}

/// The value [`open_latency_line`] printed, parsed back out of one line of the daemon's log.
/// `None` for any line that is not one of ours (including a malformed number, which a caller
/// should treat the same as "not present" rather than fail the whole log on).
pub fn parse_open_latency_ms(line: &str) -> Option<f64> {
    line.trim()
        .strip_prefix("launcher.open_latency_ms=")?
        .parse()
        .ok()
}

fn unknown_surface(surface: &SurfaceName) -> Response {
    Response::Failed(Failure {
        kind: FailureKind::UnknownSurface,
        message: format!("no surface {:?}", surface.0),
    })
}

fn unsupported(message: &str) -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Unsupported,
        message: message.to_owned(),
    })
}

fn internal(error: &impl std::fmt::Display) -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Internal,
        message: error.to_string(),
    })
}

#[cfg(test)]
mod shutdown_tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::shutdown_runtime_within;

    /// A `spawn_blocking` task that never returns — the shape of `cosmic-randr` hanging under a
    /// compositor without `zwlr_output_manager_v1` (KWin: FINDINGS G51) — does not keep
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

#[cfg(all(test, feature = "debug"))]
mod tests {
    use sill_ipc::{Point, PointerButton};

    use super::{DebugAction, to_inject};

    #[test]
    fn a_debug_click_maps_onto_shell_hosts_injection_with_its_button() {
        let at = Point { x: 1200, y: 16 };
        let cases = [
            (
                PointerButton::Primary,
                shell_host::debug::PointerButton::Primary,
            ),
            (
                PointerButton::Secondary,
                shell_host::debug::PointerButton::Secondary,
            ),
            (
                PointerButton::Middle,
                shell_host::debug::PointerButton::Middle,
            ),
        ];
        for (ours, theirs) in cases {
            assert_eq!(
                to_inject(DebugAction::Click(at, ours)).ok(),
                Some(shell_host::Inject::Click {
                    at: shell_host::Point { x: 1200, y: 16 },
                    button: theirs,
                }),
                "{ours:?}"
            );
        }
    }
}
