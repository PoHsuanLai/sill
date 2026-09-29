//! The dock's runtime: the machine in a signal, the frame drive, the one-shot timers, the
//! context menu popup and the services. The only effectful part of the dock; every decision is
//! `machine::step`, `effects`, `changes`, `context_menu` or `menu_geometry`.
//!
//! Frames follow shell-host's frame hook: the machine's `RequestFrames` takes a
//! `FrameDrive`, every frame tick of the surface feeds `Frame`, and `StopFrames` drops the drive,
//! so an idle dock renders nothing. A `Redraw` stamps the time the next render draws at.

use dioxus::prelude::*;
use shell_host::{FrameDrive, Root, SurfaceHandle};
use sill_launcher::{AppId, DesktopAction};
use sill_services::{Badges, LaunchCommand, LaunchRequest, Services, Stamp};
use sill_settings::DockMenuOrder;
use tokio::sync::mpsc;

use crate::clock::FrameClock;
use crate::hooks::use_services;
use crate::surfaces::dock::changes::{Snapshot, changes};
use crate::surfaces::dock::context_menu::{OptionHeld, context_menu, menu_entries, menu_windows};
use crate::surfaces::dock::effects::{EffectCtx, effect_commands};
use crate::surfaces::dock::frame::Frame;
use crate::surfaces::dock::items::DockItem;
use crate::surfaces::dock::machine::{DockEffect, DockEvent, DockParams, DockState, TileId, step};
use crate::surfaces::dock::menu_geometry::{menu_config, menu_placement};
use crate::surfaces::dock::menu_popup::{DockMenuPopup, MenuLink, PopupLook, PopupReport};
use crate::surfaces::dock::pick::{DockCommand, pick_commands};
use crate::surfaces::dock::pointer::{DockInput, resolve};

/// What the session decides against, from the dock's latest render.
#[derive(Clone)]
pub(crate) struct SessionDeps {
    pub items: Vec<DockItem>,
    /// The frame on screen, which pointer input is hit-tested against.
    pub frame: Frame,
    pub params: DockParams,
    pub world: World,
    pub order: DockMenuOrder,
    pub look: PopupLook,
    /// `dock.remove_threshold_px`.
    pub remove: sill_services::Cpx,
}

/// What the services said, as the effects read it.
#[derive(Clone)]
pub(crate) struct World {
    pub toplevels: sill_services::Toplevels,
    pub index: sill_services::AppIndex,
    pub pins: sill_services::DockPins,
    pub badges: Badges,
    pub workspace: Option<sill_services::WorkspaceKey>,
}

/// The session as the dock's render sees it.
#[derive(Clone, Copy)]
pub(crate) struct DockSession {
    pub feed: Callback<DockInput>,
    pub machine: Signal<DockState>,
    /// The instant the current frame is drawn at.
    pub drawn: Signal<Stamp>,
    slots: Slots,
}

impl DockSession {
    /// Hand the session this render's view of the world, and feed what changed since the last.
    pub(crate) fn update(&self, deps: SessionDeps) {
        let now = Snapshot {
            items: deps.items.clone(),
            badges: deps.world.badges.clone(),
        };
        let mut held = self.slots.deps;
        held.set(Some(deps));
        notice_changes(self.slots, self.feed, now);
    }
}

/// The mutable halves, all `Copy` handles.
#[derive(Clone, Copy)]
struct Slots {
    machine: Signal<DockState>,
    drawn: Signal<Stamp>,
    drive: CopyValue<Option<FrameDrive>>,
    menu: CopyValue<MenuOpen>,
    seen: CopyValue<Snapshot>,
    reports: CopyValue<mpsc::UnboundedSender<PopupReport>>,
    deps: CopyValue<Option<SessionDeps>>,
    /// The session's own feed, for its timers (set once the callback exists).
    feed: CopyValue<Option<Callback<DockInput>>>,
}

/// Whether the session has a context menu popup up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuOpen {
    Up,
    Down,
}

/// Where commands go: the services, and the surface that asks for activation tokens and
/// holds the frame drive (none in a test render).
#[derive(Clone)]
struct Outward {
    services: Services,
    surface: Option<SurfaceHandle>,
}

/// Start the dock's session (once per dock) with the machine at `initial`.
pub(crate) fn use_dock_session(initial: DockState) -> DockSession {
    let clock =
        use_hook(|| try_consume_context::<FrameClock>().unwrap_or_else(FrameClock::started));
    let outward = Outward {
        services: use_services(),
        surface: try_use_context::<SurfaceHandle>(),
    };
    let (reports_tx, reports_rx) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<PopupReport>();
        (tx, CopyValue::new(Some(rx)))
    });
    let slots = Slots {
        machine: use_signal(|| initial),
        drawn: use_signal(|| Stamp(0)),
        drive: use_hook(|| CopyValue::new(None)),
        menu: use_hook(|| CopyValue::new(MenuOpen::Down)),
        seen: use_hook(|| CopyValue::new(Snapshot::default())),
        reports: use_hook(|| CopyValue::new(reports_tx)),
        deps: use_hook(|| CopyValue::new(None)),
        feed: use_hook(|| CopyValue::new(None)),
    };
    let feed = use_hook(|| {
        let outward = outward.clone();
        Callback::new(move |input: DockInput| {
            let deps = slots.deps.peek();
            if let Some(deps) = deps.as_ref() {
                handle(slots, deps, &outward, clock, input);
            }
        })
    });
    let mut own = slots.feed;
    if own.peek().is_none() {
        own.set(Some(feed));
    }
    use_hook(move || start_tasks(slots, feed, outward, reports_rx));
    DockSession {
        feed,
        machine: slots.machine,
        drawn: slots.drawn,
        slots,
    }
}

/// The session's two tasks: frame ticks and popup reports.
fn start_tasks(
    slots: Slots,
    feed: Callback<DockInput>,
    outward: Outward,
    reports: CopyValue<Option<mpsc::UnboundedReceiver<PopupReport>>>,
) {
    if let Some(surface) = outward.surface.clone() {
        spawn(async move {
            loop {
                let _tick = surface.next_frame().await;
                feed.call(DockInput::Event(DockEvent::Frame));
            }
        });
    }
    let mut reports = reports;
    if let Some(mut rx) = reports.write().take() {
        spawn(async move {
            while let Some(report) = rx.recv().await {
                on_report(slots, feed, &outward, report);
            }
        });
    }
}

/// A popup report: run a pick, and tell the machine the menu closed (once).
fn on_report(slots: Slots, feed: Callback<DockInput>, outward: &Outward, report: PopupReport) {
    let mut menu = slots.menu;
    if let PopupReport::Picked(pick) = &report {
        send_all(outward, pick_commands(pick));
    }
    if *menu.peek() == MenuOpen::Up {
        menu.set(MenuOpen::Down);
        feed.call(DockInput::Event(DockEvent::MenuClosed));
    }
}

/// The service changes since the last render, fed after it.
fn notice_changes(slots: Slots, feed: Callback<DockInput>, now: Snapshot) {
    let mut seen = slots.seen;
    if *seen.peek() == now {
        return;
    }
    let events = changes(&seen.peek(), &now);
    seen.set(now);
    if !events.is_empty() {
        spawn(async move {
            for event in events {
                feed.call(DockInput::Event(event));
            }
        });
    }
}

/// Step the machine with `event` at the clock's now and carry out the effects.
fn handle(
    slots: Slots,
    deps: &SessionDeps,
    outward: &Outward,
    clock: FrameClock,
    input: DockInput,
) {
    let now = clock.now();
    let mut machine = slots.machine;
    let event = match input {
        DockInput::Event(event) => event,
        DockInput::Pointer(pointer) => resolve(
            pointer,
            &deps.frame,
            &deps.items,
            &machine.peek(),
            deps.remove,
        ),
    };
    let (next, effects) = step(machine.peek().clone(), event, now, &deps.params);
    if *machine.peek() != next {
        machine.set(next);
    }
    for effect in effects {
        apply(slots, deps, outward, clock, now, effect);
    }
}

fn apply(
    slots: Slots,
    deps: &SessionDeps,
    outward: &Outward,
    clock: FrameClock,
    now: Stamp,
    effect: DockEffect,
) {
    let mut drive = slots.drive;
    let mut drawn = slots.drawn;
    match &effect {
        DockEffect::Redraw => drawn.set(now),
        DockEffect::RequestFrames => {
            if drive.peek().is_none()
                && let Some(surface) = &outward.surface
            {
                drive.set(Some(surface.drive_frames()));
            }
        }
        DockEffect::StopFrames => {
            // The settle frame (a leave reaching rest) is drawn too.
            if *drawn.peek() != now {
                drawn.set(now);
            }
            drive.set(None);
        }
        DockEffect::WakeAt(at) => wake(*slots.feed.peek(), clock, *at),
        DockEffect::SetInput(region) => {
            if let Some(surface) = &outward.surface {
                surface.set_input_region(region.clone());
            }
        }
        DockEffect::SetBlur(blur) => {
            if let Some(surface) = &outward.surface {
                surface.set_blur(blur.clone());
            }
        }
        DockEffect::SetExclusive(zone) => {
            if let Some(surface) = &outward.surface {
                surface.set_exclusive_zone(*zone);
            }
        }
        DockEffect::OpenMenu { tile } => open_menu(slots, deps, outward, *tile),
        // The label is ds `Tooltip{Fly}`'s own (tile.rs); minimize targets need shell-host
        // support first.
        DockEffect::ShowLabel { .. } | DockEffect::HideLabel | DockEffect::SetMinimizeRects => {}
        DockEffect::Launch { .. }
        | DockEffect::Activate { .. }
        | DockEffect::ShowInFiles { .. }
        | DockEffect::Reorder { .. }
        | DockEffect::Unpin { .. }
        | DockEffect::SpringOpen { .. } => {
            let ctx = EffectCtx {
                items: &deps.items,
                toplevels: &deps.world.toplevels,
                index: &deps.world.index,
                pins: &deps.world.pins,
                workspace: deps.world.workspace,
            };
            send_all(outward, effect_commands(&effect, &ctx));
        }
    }
}

/// Feed `Tick` at `at` (the machine's one-shot timer).
fn wake(feed: Option<Callback<DockInput>>, clock: FrameClock, at: Stamp) {
    let Some(feed) = feed else { return };
    let wait = std::time::Duration::from_millis(at.since(clock.now()));
    spawn(async move {
        ds::sleep(wait).await;
        feed.call(DockInput::Event(DockEvent::Tick));
    });
}

/// Open the context menu for `tile` over its tile as drawn now.
fn open_menu(slots: Slots, deps: &SessionDeps, outward: &Outward, tile: TileId) {
    let Some(at) = deps.items.iter().position(|i| i.tile == tile) else {
        return;
    };
    let item = &deps.items[at];
    let app = item.kind.app_id().and_then(|id| deps.world.index.get(id));
    let rows = context_menu(item, app, &deps.world.toplevels, deps.order, OptionHeld::Up);
    let windows = menu_windows(item, app, &deps.world.toplevels);
    let entries = menu_entries(&rows, item, &windows, deps.order);
    let mut menu = slots.menu;
    menu.set(MenuOpen::Up);
    let reports = slots.reports.peek().clone();
    let (Some(surface), Some(area), false) = (
        &outward.surface,
        deps.frame.tiles.get(at),
        entries.is_empty(),
    ) else {
        // Nothing to show (the Trash tile): the menu closes as it opens.
        let _ = reports.send(PopupReport::Dismissed);
        return;
    };
    let placement = menu_placement(*area, &entries);
    let link = MenuLink {
        size: placement.size,
        entries: entries.clone(),
        look: deps.look,
        reports,
    };
    surface.open_popup(
        menu_config(&entries),
        placement,
        Root::new(DockMenuPopup).context(link),
    );
}

/// Send `commands`; a launch first asks shell-host for an activation token. A service that
/// has stopped drops its commands: there is nobody left to tell.
fn send_all(outward: &Outward, commands: Vec<DockCommand>) {
    let services = &outward.services;
    for command in commands {
        let _gone_is_dropped = match command {
            DockCommand::Wl(c) => services.cosmic_wl.control.send(c),
            DockCommand::Pins(c) => services.pins.control.send(c),
            DockCommand::Service(c) => services.launch.control.send(c),
            DockCommand::Launch { app, action, uris } => {
                launch(outward.clone(), app, action, uris);
                Ok(())
            }
        };
    }
}

fn launch(outward: Outward, app: AppId, action: DesktopAction, uris: Vec<String>) {
    spawn(async move {
        let token = match &outward.surface {
            Some(surface) => surface.activation_token(Some(app.0.clone())).await,
            None => None,
        };
        let _gone_is_dropped =
            outward
                .services
                .launch
                .control
                .send(LaunchCommand::Launch(LaunchRequest {
                    app,
                    action,
                    files: Vec::new(),
                    uris,
                    token,
                }));
    });
}
