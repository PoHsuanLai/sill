//! The host's events, handled: an IPC request is answered, the lock and the idle dim act on the
//! host, and every other event moves the outputs' bookkeeping (a surface the compositor took away,
//! a bar resized by a settled settings change) and then spawns whatever is missing.

use std::path::PathBuf;
use std::time::Instant;

use shell_host::{CloseReason, HostCtx, HostEvent, SurfaceId};
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_ipc::show::{LauncherRequest, ShowRequest};
use sill_model::displays::DisplaysCommand;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::links::Links;
use sill_surfaces::services::Services;
use tokio::sync::watch;

use super::msg::ShellMsg;
use super::settings;
use super::surfaces::{self, Spawn};
use crate::ipc::dispatch::{Reach, route};
use crate::ipc::latency;
use crate::outputs::{OutputAction, OutputSurfaces, Outputs, reconcile};

/// What the host's event handler holds between events.
#[derive(Debug)]
pub struct Shell {
    pub outputs: Outputs,
    pub shell_lock: crate::lock::ShellLock,
    pub services: Services,
    pub settings_feed: SettingsFeed,
    /// `None` when neither `XDG_CONFIG_HOME` nor `HOME` is set.
    pub settings_dir: Option<PathBuf>,
    pub settings: watch::Sender<ShellFile>,
    /// The links every surface root is given and the requests write.
    pub links: Links,
}

/// Every event, handled: an IPC request (needs the services and the settings), the lock's and the
/// idle dim's wishes, or everything else through [`host_event`]. The launcher panel's keyboard focus goes to the launcher link (a focus loss
/// closes it, design/06 §20.2).
pub fn handle(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, event: HostEvent<ShellMsg>) {
    match event {
        HostEvent::App(ShellMsg::Ipc { request, reply }) => {
            let _ = reply.send(answer(shell, ctx, request));
        }
        HostEvent::App(ShellMsg::Lock(wish)) => apply_lock(shell, ctx, wish),
        HostEvent::App(ShellMsg::Dim(dim)) => crate::idle_dim::apply(ctx, &shell.outputs, dim),
        other => host_event(shell, ctx, other),
    }
}

/// One IPC request's answer. `Toggle` is timed from here (the request's arrival) regardless of
/// what the dispatch does with it: the `launcher.open_latency_ms` line
/// ([`latency::open_latency_line`]). Placed at the request's arrival rather than inside the
/// launcher's dispatch so that function's body can change freely, and the elapsed time grows on
/// its own from "IPC overhead only" to "IPC arrival to the frame that presents the shown panel"
/// once the dispatch itself waits on that frame.
fn answer(shell: &Shell, ctx: &mut HostCtx<ShellMsg>, request: Request) -> Response {
    let toggled_at = matches!(
        request,
        Request::Show(ShowRequest::Launcher(LauncherRequest::Toggle))
    )
    .then(Instant::now);
    let reach = Reach {
        services: &shell.services,
        outputs: &shell.outputs,
        links: &shell.links,
        settings_dir: shell.settings_dir.as_deref(),
        settings: &shell.settings,
    };
    let response = route(ctx, &reach, request);
    if let Some(toggled_at) = toggled_at {
        latency::log_open_latency(toggled_at.elapsed());
    }
    response
}

fn apply_lock(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, wish: crate::lock::LockWish) {
    let Shell {
        outputs,
        shell_lock,
        services,
        settings_feed,
        links,
        ..
    } = shell;
    crate::lock::apply(
        ctx,
        shell_lock,
        wish,
        || surfaces::lock_factory(services, settings_feed, links),
        &links.lock,
        outputs,
        services,
    );
}

/// Any other host event: the lock follows it, the launcher and the power menu take their keyboard
/// focus, the scroll engine gets its first config, a watched settings change is published, the
/// displays service rescans, the outputs' bookkeeping moves ([`on_event`]), and whatever is
/// missing is spawned.
fn host_event(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, event: HostEvent<ShellMsg>) {
    crate::lock::on_host_event(
        ctx,
        &mut shell.shell_lock,
        &event,
        &shell.links.lock,
        &shell.services,
    );
    follow_keyboard_focus(shell, &event);
    // The engine has no config until told one; the watcher's and the reload's changes land through
    // `HostEvent::App(ShellMsg::Settings(_))` and `Request::Reload`.
    if matches!(event, HostEvent::Started) {
        ctx.set_scroll_config(crate::scroll::config(&shell.settings.borrow()));
    }
    // The watch's re-read reaches the feed (every follower: the policies, the locker, corners, the
    // focus mode) the same way `sill reload` does; geometry follows the same event just below.
    settings::publish_watched(&shell.settings, &event);
    rescan_displays(&shell.services, &event);
    on_event(ctx, &mut shell.outputs, event);
    spawn_missing(shell, ctx);
}

/// A keyboard focus change of the launcher's or the power menu's surface goes to its link: a
/// focus loss closes the launcher (design/06 §20.2).
fn follow_keyboard_focus(shell: &Shell, event: &HostEvent<ShellMsg>) {
    let HostEvent::KeyboardFocus { id, focus } = event else {
        return;
    };
    let on = |has: fn(&OutputSurfaces) -> Option<SurfaceId>| {
        shell
            .outputs
            .outputs
            .iter()
            .any(|(_, surfaces)| has(surfaces) == Some(*id))
    };
    if on(|surfaces| surfaces.launcher) {
        shell.links.launcher.focus(*focus);
    }
    if on(|surfaces| surfaces.power_menu) {
        shell.links.power_menu.focus(*focus);
    }
}

/// Spawn whatever each output lacks, then point the banners' link at the output that has a
/// banner column.
fn spawn_missing(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>) {
    let spawn = Spawn {
        services: &shell.services,
        settings_feed: &shell.settings_feed,
        settings: &shell.settings,
        links: &shell.links,
    };
    surfaces::spawn_missing(ctx, &mut shell.outputs, &spawn);
    shell
        .links
        .banners
        .set(crate::notifications::banner_output(&shell.outputs));
}

/// Outputs coming, going or changing make the displays service rescan.
fn rescan_displays(services: &Services, event: &HostEvent<ShellMsg>) {
    match event {
        HostEvent::Started
        | HostEvent::OutputAdded(_)
        | HostEvent::OutputChanged(_)
        | HostEvent::OutputRemoved(_) => {
            let _ = services.displays.control.send(DisplaysCommand::Rescan);
        }
        _ => {}
    }
}

/// The host event handler: outputs come and go, surfaces closed by the compositor are noted for
/// respawning, and a settled settings change resizes the bar. The events that need the services
/// or the lock's state are answered by [`handle`] and [`host_event`].
fn on_event(ctx: &mut HostCtx<ShellMsg>, outputs: &mut Outputs, event: HostEvent<ShellMsg>) {
    match event {
        HostEvent::Started
        | HostEvent::OutputAdded(_)
        | HostEvent::OutputChanged(_)
        | HostEvent::OutputRemoved(_) => reconcile_outputs(ctx, outputs),
        HostEvent::SurfaceClosed { id, reason } => on_surface_closed(outputs, id, reason),
        // Answered by `host_event` (`crate::lock::on_host_event`), which holds the lock.
        HostEvent::KeyboardFocus { .. } | HostEvent::Lock { .. } => {}
        HostEvent::App(ShellMsg::Settings(file)) => {
            resize_bars(ctx, outputs, &file);
            ctx.set_scroll_config(crate::scroll::config(&file));
        }
        // Answered by `handle`, which holds the lock's state.
        HostEvent::App(ShellMsg::Lock(_)) => {}
        // Answered by `handle`, which maps the idle dim surfaces.
        HostEvent::App(ShellMsg::Dim(_)) => {}
        HostEvent::App(ShellMsg::Ipc { reply, .. }) => {
            // Never reached through `handle`, which answers `Ipc` itself; answered rather
            // than dropped in case another caller ever delivers one here directly.
            let _ = reply.send(Response::Failed(Failure {
                kind: FailureKind::Internal,
                message: "ipc requests are dispatched before on_event".to_owned(),
            }));
        }
    }
}

/// `ctx.outputs()` against `outputs`, applying every action `reconcile` names: a new output's
/// row is added empty (marking it for `surfaces::spawn_missing`), a changed output's row is kept
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
