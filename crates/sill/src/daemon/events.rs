//! The host's events, handled: an IPC request is answered, the lock and the idle dim act on the
//! host, and every other event moves the outputs' bookkeeping (a surface the compositor took away,
//! a bar resized by a settled settings change) and then spawns whatever is missing.

use std::cell::RefCell;
use std::time::Instant;

use ds_settings::{Environment, Store};
use shell_host::{
    CloseReason, HostCtx, HostEvent, KeyboardFocus, LayerOp, PalmrestAction, SurfaceId,
};
use sill_bar::Bar;
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_launcher::actions::ActionId;
use sill_model::actions::Ask;
use sill_model::desktop::{DesktopCommand, MacMods};
use sill_model::displays::DisplaysCommand;
use sill_model::keyboard::{Grip, ShellKeys};
use sill_model::keycap::KeycapCommand;
use sill_settings::ShellFile;
use sill_ui_kit::links::{Asked, BannerOutput, Links};
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surface::{OutputPolicy, Surface};
use tokio::sync::watch;

use super::msg::ShellMsg;
use super::palmrest::{self, Plan};
use super::settings;
use super::surfaces::{self, Spawn};
use crate::edge_reveal::FullScreenPrefs;
use crate::ipc::dispatch::{Carried, DesktopLine, Reach, route};
use crate::ipc::latency;
use crate::outputs::{OutputAction, OutputSurfaces, Outputs, reconcile};
use sill_ui_kit::shell_motion::shell_motion;

/// What the host's event handler holds between events.
#[derive(Debug)]
pub struct Shell {
    pub outputs: Outputs,
    pub shell_lock: crate::lock::ShellLock,
    pub services: Services,
    pub settings_feed: SettingsFeed,
    /// The settings file's store.
    pub store: Store,
    pub settings: watch::Sender<ShellFile>,
    /// The person's styles and their crash guard, which every surface root draws.
    pub styles: settings::StyleKeeper,
    /// The desktop's appearance, which every surface root draws.
    pub appearance: watch::Sender<Environment>,
    /// The links every surface root is given and the requests write.
    pub links: Links,
    /// Which of the shell's surfaces hold the keyboard, told to the keycap service.
    pub keys: ShellKeys,
    /// The desktop's own compositor's arm, when the host is on the connection it handed over.
    pub seat: super::compositor::ShellSeat,
    /// Where the service manager is told the shell has started.
    pub notify: crate::watchdog::Notifier,
    /// The async runtime the services run on, for the work an IPC request starts off the host
    /// thread (`sill launch` waits for the app's process).
    pub runtime: tokio::runtime::Handle,
    /// The windows Show Desktop put away, to bring back on its next firing.
    pub stash: RefCell<crate::actions::show_desktop::Stash>,
    /// The output the sheets' current questions were started on.
    pub sheet_latch: crate::sheet_latch::SheetLatch,
    /// The modifiers held over the shell's surfaces now (the compositor's last
    /// `ShellNote::Modifiers`), which `corners::spawn_hits` reads at a firing.
    pub held_mods: watch::Sender<MacMods>,
}

/// Every event, handled: an IPC request (needs the services and the settings), the lock's and the
/// idle dim's wishes, or everything else through [`host_event`]. The launcher panel's keyboard focus goes to the launcher link (a focus loss
/// closes it, design/06 §20.2).
pub fn handle(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, event: HostEvent<ShellMsg>) {
    match event {
        HostEvent::App(ShellMsg::Ipc {
            request: Request::Act(ActRequest::Launch(app)),
            reply,
        }) => {
            // Answered once the app's process exists, off this thread.
            crate::launch::ask(
                ctx,
                &shell.services,
                &shell.outputs,
                &shell.runtime,
                app,
                reply,
            );
        }
        HostEvent::App(ShellMsg::Ipc {
            request:
                Request::Do(Ask {
                    action: ActionId::NextInputSource,
                    ..
                }),
            reply,
        }) => {
            // Answered once the service says how the switch went, off this thread.
            crate::actions::input_source::ask(&shell.services, &shell.runtime, reply);
        }
        HostEvent::App(ShellMsg::Action(asked)) => {
            let Asked { ask, origin } = asked.clone();
            let response = reached(shell, ctx, |ctx, reach| {
                crate::actions::perform(ctx, reach, ask, origin)
            });
            crate::actions::report(&asked, &response);
        }
        HostEvent::App(ShellMsg::Ipc { request, reply }) => {
            let _ = reply.send(answer(shell, ctx, request));
        }
        HostEvent::Palmrest(ask) => palmrest_ask(shell, ctx, &ask),
        HostEvent::App(ShellMsg::Lock(wish)) => apply_lock(shell, ctx, wish),
        HostEvent::App(ShellMsg::Sheets) => {
            let latch = std::mem::take(&mut shell.sheet_latch);
            shell.sheet_latch = latch.follow(&shell.services, &shell.links, &shell.outputs);
        }
        HostEvent::App(ShellMsg::Dim(dim)) => crate::idle_dim::apply(ctx, &shell.outputs, dim),
        HostEvent::App(ShellMsg::Summon(out)) => {
            crate::companion::apply(&shell.outputs, &shell.services, &shell.links.launcher, out)
        }
        HostEvent::App(ShellMsg::Note(note)) => super::compositor::apply(shell, note),
        HostEvent::App(ShellMsg::DockRects(rects)) => shell.seat.set_rects(rects),
        HostEvent::App(ShellMsg::Command(command)) => {
            // Without a running arm (a session without our compositor) there is nowhere to go;
            // a request for a picture is then answered as failed, so its question is not left
            // waiting.
            let asked = command.clone();
            if !shell.seat.command(command) {
                crate::eyedropper::refuse(
                    &shell.links.eyedropper,
                    &shell.services.capture.control,
                    &asked,
                );
            }
        }
        HostEvent::App(ShellMsg::Desktop) => {
            shell.seat.desktop_changed();
            follow_full_screen(shell);
        }
        other => host_event(shell, ctx, other),
    }
}

/// A gesture's action that needs the session, asked by palmrest (a system service): carried out as
/// the person's own command would be. None while the lock screen is up (palmrest drops them then
/// too).
fn palmrest_ask(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, ask: &PalmrestAction) {
    if shell.shell_lock.id.is_some() {
        return;
    }
    match palmrest::plan(ask) {
        Plan::Request(request) => {
            if let Response::Failed(failure) = answer(shell, ctx, request) {
                eprintln!("sill: palmrest: {}", failure.message);
            }
        }
        Plan::Program { program, args } => palmrest::run(&program, &args),
        Plan::Refused(why) => eprintln!("sill: palmrest: {ask:?}: {why}"),
    }
}

/// One IPC request's answer. `Toggle` is timed from here (the request's arrival) regardless of
/// what the dispatch does with it: the `launcher.open_latency_ms` line
/// ([`latency::open_latency_line`]). Placed at the request's arrival rather than inside the
/// launcher's dispatch so that function's body can change freely, and the elapsed time grows on
/// its own from "IPC overhead only" to "IPC arrival to the frame that presents the shown panel"
/// once the dispatch itself waits on that frame.
fn answer(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, request: Request) -> Response {
    let toggled_at = matches!(
        request,
        Request::Do(Ask {
            action: ActionId::Launcher,
            ..
        })
    )
    .then(Instant::now);
    let response = reached(shell, ctx, |ctx, reach| route(ctx, reach, request));
    if let Some(toggled_at) = toggled_at {
        latency::log_open_latency(toggled_at.elapsed());
    }
    response
}

/// `then` run with what a request or an action may reach.
fn reached<T>(
    shell: &mut Shell,
    ctx: &mut HostCtx<ShellMsg>,
    then: impl FnOnce(&mut HostCtx<ShellMsg>, &Reach) -> T,
) -> T {
    let seat = RefCell::new(&mut shell.seat);
    let carry = |command: DesktopCommand| {
        if seat.borrow_mut().command(command) {
            Carried::Yes
        } else {
            Carried::No
        }
    };
    let reach = Reach {
        services: &shell.services,
        outputs: &shell.outputs,
        links: &shell.links,
        store: &shell.store,
        settings: &shell.settings,
        styles: &shell.styles,
        desktop: DesktopLine(&carry),
        runtime: &shell.runtime,
        stash: &shell.stash,
    };
    then(ctx, &reach)
}

fn apply_lock(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>, wish: crate::lock::LockWish) {
    let Shell {
        outputs,
        shell_lock,
        services,
        settings_feed,
        links,
        settings,
        ..
    } = shell;
    crate::lock::apply(
        ctx,
        shell_lock,
        wish,
        || surfaces::lock_factory(services, settings_feed, links, &settings.borrow()),
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
    tell_keycap_the_holders(shell, &event);
    super::compositor::follow_mapped(&mut shell.seat, &shell.outputs, ctx, &event);
    // The engine has no config until told one; the watcher's and the reload's changes land through
    // `HostEvent::App(ShellMsg::Settings(_))` and `Request::Reload`.
    if matches!(event, HostEvent::Started) {
        shell.notify.ready();
        ctx.set_scroll_config(crate::scroll::config(&shell.settings.borrow()));
    }
    // The watch's re-read reaches the feed (every follower: the policies, the locker, corners, the
    // focus mode) the same way `sill reload` does; geometry follows the same event just below.
    settings::publish_watched(&shell.settings, &event);
    shell.styles.publish(&event);
    settings::publish_watched_appearance(&shell.appearance, &event);
    rescan_displays(&shell.services, &event);
    on_event(ctx, &mut shell.outputs, event);
    follow_full_screen(shell);
    spawn_missing(shell, ctx);
    crate::work_areas::publish(&shell.services, &shell.outputs, &shell.settings.borrow());
    crate::workspace_looks::publish(&shell.services, &shell.settings.borrow());
}

/// Full screen as the settings, the appearance and the desktop now have it: the compositor hears
/// what the bar does there and how much motion is allowed (only what changed is sent), and the
/// bars hear which outputs show a full-screen Space.
fn follow_full_screen(shell: &mut Shell) {
    let prefs = {
        let file = shell.settings.borrow();
        let environment = shell.appearance.borrow();
        FullScreenPrefs::of(file.bar.autohide, shell_motion(&environment))
    };
    shell.seat.set_prefs(prefs);
    let outputs = &shell.outputs;
    let full = crate::edge_reveal::full_screens(&shell.seat.view(), |name| outputs.named(name));
    shell.links.edges.set_full_screen(full);
}

/// A keyboard focus change goes to the surface's own route, which its `Surface::KEYBOARD`
/// declared: a launcher, a power menu, a notification center, a switcher or a split picker closes
/// when it loses the keyboard (design/06 §20.2), a question that waits for an answer stays.
fn follow_keyboard_focus(shell: &Shell, event: &HostEvent<ShellMsg>) {
    let HostEvent::KeyboardFocus { id, focus } = event else {
        return;
    };
    let surface = shell
        .outputs
        .outputs
        .iter()
        .find_map(|(_, surfaces)| surfaces.spawned_by(*id));
    if let Some(surface) = surface {
        surface.focus.tell(*focus);
    }
}

/// Which of the shell's surfaces hold the keyboard, kept from the host's focus and close events;
/// when whether any does changes, the keycap service hears it (it then sends `keymap-aware`, or
/// the window's profile again).
fn tell_keycap_the_holders(shell: &mut Shell, event: &HostEvent<ShellMsg>) {
    let before = shell.keys.focus();
    let keys = std::mem::take(&mut shell.keys);
    shell.keys = match event {
        HostEvent::KeyboardFocus { id, focus } => keys.after(
            id.0,
            match focus {
                KeyboardFocus::Gained => Grip::Gained,
                KeyboardFocus::Lost => Grip::Lost,
            },
        ),
        HostEvent::SurfaceClosed { id, .. } => keys.closed(id.0),
        _ => keys,
    };
    let after = shell.keys.focus();
    if after != before {
        let _ = shell
            .services
            .keycap
            .control
            .send(KeycapCommand::ShellFocus(after));
    }
}

/// Spawn whatever each output lacks, then point the banners' link at the output that has a
/// banner column and the sheets' link at the latched output (or the first one, when that went).
fn spawn_missing(shell: &mut Shell, ctx: &mut HostCtx<ShellMsg>) {
    let spawn = Spawn {
        services: &shell.services,
        settings_feed: &shell.settings_feed,
        settings: &shell.settings,
        links: &shell.links,
    };
    surfaces::spawn_missing(ctx, &mut shell.outputs, &spawn);
    shell.links.banners.set(
        shell
            .outputs
            .pick(OutputPolicy::Main, None)
            .map_or(BannerOutput::Nowhere, BannerOutput::On),
    );
    shell
        .links
        .sheets
        .set(shell.sheet_latch.output(&shell.outputs));
    shell.links.lock.set(crate::lock::prompt_among(
        shell.links.lock.now(),
        &shell.outputs,
    ));
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
        // A surface's first frame after a map; nothing here waits on it yet.
        HostEvent::SurfaceMapped { .. } => {}
        // The shell is not an app the person quits: it stays up, and logging out stops its unit.
        HostEvent::QuitAsked => {}
        // Answered by `handle`, which carries it out (`palmrest_ask`).
        HostEvent::Palmrest(_) => {}
        HostEvent::App(ShellMsg::Settings(file)) => {
            resize_bars(ctx, outputs, &file);
            ctx.set_scroll_config(crate::scroll::config(&file));
        }
        // Published by `host_event` to the feed the surfaces draw it from; no geometry follows.
        HostEvent::App(
            ShellMsg::Style(..) | ShellMsg::Look | ShellMsg::TurnOnStyles | ShellMsg::Appearance(_),
        ) => {}
        // Answered by `handle`, which holds the lock's state.
        HostEvent::App(ShellMsg::Lock(_)) => {}
        // Answered by `handle`, which maps the idle dim surfaces.
        HostEvent::App(ShellMsg::Dim(_)) => {}
        // Answered by `handle`, which holds the launcher link and the outputs.
        HostEvent::App(ShellMsg::Summon(_)) => {}
        // Answered by `handle`, which holds the links and the services.
        HostEvent::App(ShellMsg::Note(_)) => {}
        // Answered by `handle`, which holds the sheet latch and the links.
        HostEvent::App(ShellMsg::Sheets) => {}
        // Answered by `handle`, which carries it out through `actions::perform`.
        HostEvent::App(ShellMsg::Action(_)) => {}
        // Answered by `handle`, which holds the seat.
        HostEvent::App(ShellMsg::DockRects(_) | ShellMsg::Desktop) => {}
        // Answered by `handle`, which sends it to the compositor through the services.
        HostEvent::App(ShellMsg::Command(_)) => {}
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
/// its output's row without it, which `spawn_surface` treats as "needs one".
fn on_surface_closed(outputs: &mut Outputs, id: SurfaceId, reason: CloseReason) {
    let Some((info, surfaces)) = outputs.row_with(id) else {
        return;
    };
    let name = surfaces
        .spawned_by(id)
        .map_or("surface", |surface| surface.name);
    if reason != CloseReason::Requested {
        eprintln!(
            "sill: the {name} on {:?} closed unexpectedly ({reason:?}); respawning",
            info.name
        );
    }
    surfaces.clear(id);
}

/// A settled `settings.toml`: re-apply every bar's reserved height (only `bar.height_px` can
/// change it). `spawn_surface` resizes the layer itself; the rest is drawn by the bar's own
/// component.
fn resize_bars(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs, file: &ShellFile) {
    for (info, surfaces) in &outputs.outputs {
        let exclusive = Bar::layer(info, file, ()).exclusive;
        for bar in surfaces.spawned.iter().filter(|s| s.name == Bar::NAME) {
            ctx.apply(bar.id, LayerOp::SetExclusiveZone(exclusive));
        }
    }
}
