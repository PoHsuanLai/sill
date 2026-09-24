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

use shell_host::scale::{Size as BufferSize, buffer_size};
use shell_host::{CloseReason, Host, HostConfig, HostCtx, HostEvent, OutputInfo, Root, SurfaceId};
use sill_ipc::{
    DebugRequest, DockRequest, Failure, FailureKind, LauncherRequest, Request, Response,
    SurfaceName,
};
use sill_launcher::AppId;
use sill_services::{OutputName, PinAt, PinEvent, Services, WallpaperCommand};
use sill_settings::{SettingsFeed, ShellFile};
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
    let result = host.run(move |ctx, event| {
        run_event(
            ctx,
            &mut outputs,
            &services,
            &settings_feed,
            settings_dir.as_deref(),
            &settings_tx,
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
/// else via `on_event` followed by spawning whatever it left missing.
fn run_event(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    event: HostEvent<ShellMsg>,
) {
    match event {
        HostEvent::App(ShellMsg::Ipc { request, reply }) => {
            let response =
                dispatch_request(ctx, outputs, services, settings_dir, settings_tx, request);
            let _ = reply.send(response);
        }
        other => {
            on_event(ctx, outputs, other);
            spawn_missing_bars(ctx, outputs, services, settings_feed, settings_tx);
            spawn_missing_wallpapers(ctx, outputs, services, settings_feed);
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
/// [`spawn_missing_bars`]).
fn spawn_missing_wallpapers(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    services: &Services,
    settings_feed: &SettingsFeed,
) {
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

/// `root` with the contexts every surface the daemon spawns is given: the services, the
/// settings, and the rect measurer quire's floating components read through (without it the
/// first `use_rect` read inside a render panics; FINDINGS Q10, F76). Every spawn goes through
/// here, so a new surface cannot forget one.
fn surface_root(root: Root, services: &Services, settings_feed: &SettingsFeed) -> Root {
    root.context(services.clone())
        .context(settings_feed.clone())
        .context(sill_surfaces::MEASURE)
}

/// Answer one IPC request.
fn dispatch_request(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &Outputs,
    services: &Services,
    settings_dir: Option<&Path>,
    settings_tx: &watch::Sender<ShellFile>,
    request: Request,
) -> Response {
    match request {
        Request::Ping => Response::Pong,
        Request::Launcher(request) => dispatch_launcher(request),
        Request::Dock(request) => dispatch_dock(services, request),
        Request::Reload => dispatch_reload(settings_dir, settings_tx),
        Request::Quit => {
            ctx.exit();
            Response::Done
        }
        Request::Debug(request) => dispatch_debug(ctx, outputs, request),
    }
}

/// The launcher has no surface before M4; nothing to toggle yet.
fn dispatch_launcher(_request: LauncherRequest) -> Response {
    unsupported("the launcher is not built yet (M4)")
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
fn apply_injection(_ctx: &mut HostCtx<ShellMsg>, _id: SurfaceId, _action: DebugAction) -> Response {
    unsupported("built without the debug feature")
}

/// Every known bar's and wallpaper's frame statistics, through the always-public
/// `HostCtx::take_stats` (needs neither the `debug` feature nor shell-host's `debug-inject`;
/// draining, so each report covers only the time since the previous one).
fn collect_stats(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<sill_ipc::SurfaceStats> {
    let ids: Vec<(String, SurfaceId)> =
        outputs
            .outputs
            .iter()
            .filter_map(|(info, surfaces)| Some((bar_namespace(info), surfaces.bar?)))
            .chain(outputs.outputs.iter().filter_map(|(info, surfaces)| {
                Some((wallpaper_namespace(info), surfaces.wallpaper?))
            }))
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
