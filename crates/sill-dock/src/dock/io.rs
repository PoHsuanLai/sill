//! The dock's runtime: the machine in a signal, the frame drive, the one-shot timers, the
//! context menu and the services. The only effectful part of the dock; every decision is
//! `machine::step`, `effects`, `changes`, `context_menu` or `menu_geometry`.
//!
//! Frames follow shell-host's frame hook: the machine's `RequestFrames` takes a
//! `FrameDrive`, every frame tick of the surface feeds `Frame`, and `StopFrames` drops the drive,
//! so an idle dock renders nothing. A `Redraw` stamps the time the next render draws at.

use dioxus::prelude::*;
use ds::prelude::MenuItem;
use ds::base::time::clock::sleep;
use ds::base::time::stamp::Stamp;
use shell_host::{FrameDrive, Rect, SurfaceHandle};
use sill_model::badges::Badges;
use sill_settings::{AutoHide, DockMenuOrder, Magnification};
use sill_ui_kit::services::Services;

use crate::dock::context_menu::{OptionHeld, app, divider, menu_entries, menu_windows, trash};
use crate::dock::effects::{EffectCtx, effect_commands};
use crate::dock::frame::Frame;
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{DockEffect, DockEvent, DockParams, DockState, Point, TileId, step};
use crate::dock::menu::DockMenu;
use crate::dock::menu_geometry::{GEOM, pill_anchor, tile_anchor};
use crate::dock::pick::{DockCommand, DockPick, pick_commands};
use crate::dock::pointer::{DockInput, resolve};
use crate::dock::system::{change_login, empty_trash_availability, login_state};
use ds::base::time::stamp::FrameClock;
use sill_ui_kit::hooks::{use_link, use_services};
use sill_ui_kit::links::{ActionLink, Origin};
use sill_ui_kit::menu::{ContextMenu, PopupLook, use_context_menu_closing};

/// What the session decides against, from the dock's latest render.
#[derive(Clone)]
pub(crate) struct SessionDeps {
    pub items: Vec<DockItem>,
    /// The frame on screen, which pointer input is hit-tested against.
    pub frame: Frame,
    pub params: DockParams,
    pub world: World,
    pub order: DockMenuOrder,
    /// `dock.autohide` and `dock.magnification`, which the dock's own menu reads.
    pub autohide: AutoHide,
    pub magnification: Magnification,
    pub look: PopupLook,
    /// `dock.remove_threshold_px`.
    pub remove: sill_model::units::Cpx,
}

/// What the services said, as the effects read it.
#[derive(Clone)]
pub(crate) struct World {
    pub toplevels: sill_model::windows::Toplevels,
    pub index: sill_model::apps::AppIndex,
    pub pins: sill_model::pins::DockPins,
    pub badges: Badges,
    pub workspace: Option<sill_model::ids::WorkspaceKey>,
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
    /// Hand the session this render's view of the world.
    pub(crate) fn update(&self, deps: SessionDeps) {
        let mut held = self.slots.deps;
        held.set(Some(deps));
    }
}

/// The mutable halves, all `Copy` handles.
#[derive(Clone, Copy)]
struct Slots {
    machine: Signal<DockState>,
    drawn: Signal<Stamp>,
    drive: CopyValue<Option<FrameDrive>>,
    menu: CopyValue<ContextMenu<DockMenu>>,
    deps: CopyValue<Option<SessionDeps>>,
    /// The session's own feed, for its timers (set once the callback exists).
    feed: CopyValue<Option<Callback<DockInput>>>,
}

/// Where commands go: the services, and the surface that asks for activation tokens and
/// holds the frame drive (none in a test render).
#[derive(Clone)]
struct Outward {
    services: Services,
    surface: Option<SurfaceHandle>,
    /// Where the dock asks for an action of the shell, when the daemon gave it one.
    actions: Option<ActionLink>,
}

/// Start the dock's session (once per dock) with the machine at `initial`.
pub(crate) fn use_dock(initial: DockState) -> DockSession {
    let clock =
        use_hook(|| try_consume_context::<FrameClock>().unwrap_or_else(FrameClock::started));
    let outward = Outward {
        services: use_services(),
        surface: try_use_context::<SurfaceHandle>(),
        actions: use_link::<ActionLink>(),
    };
    // The session's own feed, for its timers and the menu's close (set once the callback exists).
    let own_feed = use_hook(|| CopyValue::new(None::<Callback<DockInput>>));
    let menu = use_context_menu_closing::<DockMenu>(
        "dock-menu",
        {
            let outward = outward.clone();
            move |pick| send_all(&outward, pick_commands(&pick))
        },
        move || close_menu(own_feed),
    );
    let slots = Slots {
        machine: use_signal(|| initial),
        drawn: use_signal(|| Stamp(0)),
        drive: use_hook(|| CopyValue::new(None)),
        menu: use_hook(|| CopyValue::new(menu)),
        deps: use_hook(|| CopyValue::new(None)),
        feed: own_feed,
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
    use_hook(move || start_frame_task(feed, outward));
    DockSession {
        feed,
        machine: slots.machine,
        drawn: slots.drawn,
        slots,
    }
}

/// The session's frame task: each frame tick of the surface feeds `Frame`.
fn start_frame_task(feed: Callback<DockInput>, outward: Outward) {
    if let Some(surface) = outward.surface {
        spawn(async move {
            loop {
                let _tick = surface.next_frame().await;
                feed.call(DockInput::Event(DockEvent::Frame));
            }
        });
    }
}

/// Tell the machine its menu closed. Always from a task, never from inside a step.
fn close_menu(feed: CopyValue<Option<Callback<DockInput>>>) {
    spawn(async move {
        let feed = *feed.peek();
        if let Some(feed) = feed {
            feed.call(DockInput::Event(DockEvent::MenuClosed));
        }
    });
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
        DockEffect::OpenDividerMenu { at } => open_divider_menu(slots, deps, outward, *at),
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
        sleep(wait).await;
        feed.call(DockInput::Event(DockEvent::Tick));
    });
}

/// Open the context menu for `tile` over its tile as drawn now.
fn open_menu(slots: Slots, deps: &SessionDeps, outward: &Outward, tile: TileId) {
    let Some(at) = deps.items.iter().position(|i| i.tile == tile) else {
        return;
    };
    let item = &deps.items[at];
    let entry = item.kind.app_id().and_then(|id| deps.world.index.get(id));
    let toplevels = &deps.world.toplevels;
    let (rows, windows) = match item.kind {
        ItemKind::Trash => (trash::rows(empty_trash_availability()), Vec::new()),
        ItemKind::Pinned(_) | ItemKind::Running(_) | ItemKind::Unmatched(_) => (
            app::rows(
                item,
                entry,
                toplevels,
                deps.order,
                OptionHeld::Up,
                login_state(item.kind.app_id()),
            ),
            menu_windows(item, entry, toplevels),
        ),
    };
    let entries = menu_entries(&rows, item.kind.app_id(), &windows, deps.order);
    let anchor = deps.frame.tiles.get(at).map(|area| tile_anchor(*area));
    show(slots, deps, outward, entries, anchor);
}

/// Open the dock's own menu over the pill at `at`.
fn open_divider_menu(slots: Slots, deps: &SessionDeps, outward: &Outward, at: Point) {
    let rows = divider::rows(deps.autohide, deps.magnification);
    let entries = menu_entries(&rows, None, &[], deps.order);
    let anchor = Some(pill_anchor(deps.frame.pill, at.x));
    show(slots, deps, outward, entries, anchor);
}

/// Open the menu of `entries` over `anchor`; with nothing to show, or no surface, or a menu
/// already up, it closes as it opens.
fn show(
    slots: Slots,
    deps: &SessionDeps,
    outward: &Outward,
    entries: Vec<MenuItem<DockPick>>,
    anchor: Option<Rect>,
) {
    let menu = slots.menu.peek().clone();
    let (Some(anchor), Some(_), true, false) = (
        anchor,
        &outward.surface,
        menu.is_free(),
        entries.is_empty(),
    ) else {
        close_menu(slots.feed);
        return;
    };
    menu.open(DockMenu { entries }, deps.look, GEOM, anchor);
}

/// Send `commands`: the shell's own actions to the daemon, which asks this surface for the
/// activation token of a launch. A service that has stopped drops its commands: there is
/// nobody left to tell.
fn send_all(outward: &Outward, commands: Vec<DockCommand>) {
    let services = &outward.services;
    for command in commands {
        let _gone_is_dropped = match command {
            DockCommand::Wl(c) => services.windows.control.send(c),
            DockCommand::Pins(c) => services.pins.control.send(c),
            DockCommand::AtLogin(change) => {
                change_login(change);
                Ok(())
            }
            DockCommand::Do(ask) => {
                // The daemon carries it out; the dock's surface is where the person clicked.
                if let Some(actions) = &outward.actions {
                    actions.ask(
                        ask,
                        Origin {
                            surface: outward.surface.as_ref().map(SurfaceHandle::id),
                            output: None,
                        },
                    );
                }
                Ok(())
            }
        };
    }
}
