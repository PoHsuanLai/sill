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

    let config = HostConfig {
        first_mouse_window_ms: shell_host::config::Ms(u32::from(
            initial_settings.menus.first_mouse_window_ms.0,
        )),
        ..HostConfig::default()
    };
    let host = match Host::<ShellMsg>::connect(config) {
        Ok(host) => host,
        Err(error) => return fatal(&format!("connecting to the compositor: {error}")),
    };
    let handle = host.handle();

    // Detached: it accepts connections until the process exits with the host (doc on
    // `ipc_server::spawn`).
    let _ipc_thread = ipc_server::spawn(door, handle.clone());

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
    let result = host.run(move |ctx, event| {
        run_event(
            ctx,
            &mut outputs,
            &services,
            &settings_feed,
            settings_dir.as_deref(),
            &settings_tx,
            &launcher,
            event,
        );
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fatal(&format!("{error}")),
    }
}

fn fatal(message: &str) -> ExitCode {
    eprintln!("sill: {message}");
    ExitCode::FAILURE
}

/// Every event, dispatched: an IPC request (needs `services` and the settings), or everything
/// else via `on_event` followed by spawning whatever it left missing. The launcher panel's
/// keyboard focus goes to `launcher` (a focus loss closes it, design/06 §20.2).
#[allow(clippy::too_many_arguments)]
fn run_event(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    launcher: &LauncherLink,
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
                request,
            );
            if let Some(toggled_at) = toggled_at {
                log_open_latency(toggled_at.elapsed());
            }
            let _ = reply.send(response);
        }
        other => {
            if let HostEvent::KeyboardFocus { id, focus } = &other
                && outputs.outputs.iter().any(|(_, s)| s.launcher == Some(*id))
            {
                launcher.focus(*focus);
            }
            on_event(ctx, outputs, other);
            spawn_missing_bars(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_wallpapers(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_docks(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_launchers(ctx, outputs, services, settings_feed, settings_tx, launcher);
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
        HostEvent::KeyboardFocus { .. } => {}
        HostEvent::App(ShellMsg::Settings(file)) => resize_bars(ctx, outputs, &file),
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
fn spawn_hidden(
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
fn surface_root(root: Root, services: &Services, settings_feed: &SettingsFeed) -> Root {
    root.context(services.clone())
        .context(settings_feed.clone())
        .context(sill_surfaces::MEASURE)
        .context(sill_surfaces::FOCUS)
}

/// Answer one IPC request.
fn dispatch_request(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &Outputs,
    services: &Services,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    launcher: &LauncherLink,
    request: Request,
) -> Response {
    match request {
        Request::Ping => Response::Pong,
        Request::Launcher(request) => dispatch_launcher(outputs, launcher, request),
        Request::Dock(request) => dispatch_dock(services, request),
        Request::Reload => dispatch_reload(settings_dir, settings_tx),
        Request::Quit => {
            ctx.exit();
            Response::Done
        }
        Request::Debug(request) => dispatch_debug(ctx, outputs, request),
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

/// Every known bar's, wallpaper's, dock's and launcher surface's frame statistics, through the
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
            bar.into_iter()
                .chain(wallpaper)
                .chain(dock)
                .chain(launcher)
                .chain(catcher)
        })
        .collect();
    ids.into_iter()
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
