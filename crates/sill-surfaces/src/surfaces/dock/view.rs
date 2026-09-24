//! The dock's render: read the services and settings, hand the session this render's world,
//! lay the frame out from the machine's displayed progress and draw it (design/10 §10.3.1,
//! §10.3.12). The surface is transparent everywhere but the pill and the tiles. The pill is the
//! `Ds` root itself: a `Material::Dock` root in a box at the pill's rectangle, which draws the
//! Space gradient at the Dock's tint alpha with the material's edge, drop and radius (quire's
//! tinted frame). Everything else hangs from the root in a layer moved back to the surface's
//! origin, so tiles, dots and the hit elements keep surface coordinates. The root's content is
//! on paper (`Ground::Paper`): the tile plates are neutral paper (design/08 §4.1), and the dot
//! and separator name the frame inks themselves.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Ground, Material, SpaceLook};
use shell_host::{LayerConfig, OutputInfo, SurfaceHandle};
use sill_services::{Activity, Cpx, OutputName, WorkspaceKey, Workspaces};
use sill_settings::{AutoHide, SpacesSettings};

use crate::hooks::{
    use_appearance, use_apps, use_launcher_entry, use_pins, use_settings, use_space_look,
    use_toplevels, use_workspaces,
};
use crate::surfaces::bar::workspaces::group_for;
use crate::surfaces::dock::frame::{Area, Frame, Rows, frame};
use crate::surfaces::dock::items::dock_items;
use crate::surfaces::dock::layer::dock_layer;
use crate::surfaces::dock::machine::{
    DockParams, DockState, LabelGate, Reveal, displayed, label_gate, lift, slide_offset,
};
use crate::surfaces::dock::menu_popup::PopupLook;
use crate::surfaces::dock::pointer::{DockInput, PointerInput, button, modifier, point};
use crate::surfaces::dock::session::{SessionDeps, World, use_dock_session};
use crate::surfaces::dock::tile::Tile;

/// The dock's own stylesheet (lint-clean under `Profile::Strict`, `tests/dock_lint.rs`).
pub(crate) const DOCK_CSS: &str = include_str!("../../style/dock.css");

/// The look and key of the active workspace on `output` (design/21 §5 rule 6), the first
/// preset while there is none.
fn active(
    workspaces: &Workspaces,
    output: Option<&OutputName>,
    spaces: &SpacesSettings,
) -> (SpaceLook, Option<WorkspaceKey>) {
    let active = group_for(&workspaces.groups, output).and_then(|group| {
        group
            .workspaces
            .iter()
            .position(|w| w.activity == Activity::Active)
            .map(|index| (group.workspaces[index].key, index))
    });
    let (key, index) = active.unwrap_or((WorkspaceKey(0), 0));
    (use_space_look(key, index, spaces), active.map(|(k, _)| k))
}

/// The dock on `output`.
pub(crate) fn dock(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let (look, workspace) = active(&use_workspaces()(), on.as_ref(), &file.spaces);
    let appearance = environment.settings.appearance.appearance();
    let level = ds::resolve(appearance, look.theme, environment.system).motion;
    let params = DockParams::new(&file.dock, &file.sound, level);
    let world = World {
        toplevels: use_toplevels()(),
        index: use_apps()(),
        pins: use_pins()(),
        badges: use_launcher_entry()(),
        workspace,
    };
    let items = dock_items(&world.pins, &world.toplevels, &world.index, file.dock.trash);
    let initial = DockState::new(match file.dock.autohide {
        AutoHide::Off => Reveal::Fixed,
        AutoHide::On => Reveal::Hidden,
    });
    let session = use_dock_session(initial);
    follow_layer(dock_layer(&output, &file.dock));
    let machine = session.machine.read().clone();
    let gate = label_gate(&machine);
    let mut labels = use_context_provider(|| Signal::new(LabelGate::Open));
    if *labels.peek() != gate {
        labels.set(gate);
    }
    let at = (session.drawn)();
    let (progress, x) = displayed(&machine, at, &params);
    let width = output.logical.width;
    let shown = frame(&items, &file.dock, width, progress, x);
    let lifts: Vec<Cpx> = items
        .iter()
        .map(|item| {
            machine
                .bounces
                .iter()
                .find(|(tile, _)| *tile == item.tile)
                .map_or(Cpx(0), |(_, bounce)| lift(bounce, at, &params))
        })
        .collect();
    let slide = slide_offset(&machine.reveal, at, &params);
    let rest = frame(&items, &file.dock, width, sill_services::Permille::ZERO, x);
    session.update(SessionDeps {
        items: items.clone(),
        frame: shown.clone(),
        params,
        world: world.clone(),
        order: file.dock.context_menu_order,
        look: PopupLook {
            appearance,
            system: environment.system,
            tint: environment.tint_alpha(),
        },
        remove: Cpx::px(i32::from(file.dock.remove_threshold_px.0)),
    });
    let feed = session.feed;
    let send = move |input: PointerInput| feed.call(DockInput::Pointer(input));
    let rows = Rows::new(&file.dock);
    let strip = strip_area(
        &rest,
        &rows,
        i32::from(file.dock.autohide_trigger_strip_px.0),
        slide,
    );
    rsx! {
        div {
            id: "dock-pill",
            class: "sill-dock-pill",
            style: "{area_style(&shown.pill)};transform:translateY({px(slide)}px)",
            Ds {
                appearance,
                system: environment.system,
                look,
                material: Material::Dock,
                ground: Some(Ground::Paper),
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                style { {DOCK_CSS} }
                div {
                    class: "sill-dock-body",
                    style: "height:{px(shown.pill.height)}px",
                }
                div {
                    class: "sill-dock",
                    style: surface_style(&shown.pill, width, rows.surface),
                    onmouseenter: move |event| send(PointerInput::Enter(point(&event))),
                    onmousemove: move |event| send(PointerInput::Move(point(&event))),
                    onmouseleave: move |_| send(PointerInput::Leave),
                    onmousedown: move |event| {
                        if let Some(b) = button(&event) {
                            send(PointerInput::Press(b, point(&event)));
                        }
                    },
                    onmouseup: move |event| {
                        if let Some(b) = button(&event) {
                            send(PointerInput::Release(b, point(&event), modifier(&event)));
                        }
                    },
                    div { id: "dock-hit", class: "sill-dock-hit", style: area_style(&shown.hit) }
                    if let Some(sep) = shown.separator {
                        div {
                            class: "sill-dock-separator",
                            style: separator_style(sep, &rows, file.dock.separator_height_px.0),
                        }
                    }
                    for ((item, area), lift) in items.iter().zip(&shown.tiles).zip(lifts) {
                        Tile {
                            key: "{item.tile.0}",
                            item: item.clone(),
                            left: area.left,
                            size: area.width,
                            lift,
                            badge: item.kind.app_id().and_then(|id| world.badges.apps.get(id)).copied(),
                        }
                    }
                    if file.dock.autohide == AutoHide::On {
                        div { id: "dock-strip", class: "sill-dock-hit", style: area_style(&strip) }
                    }
                }
            }
        }
    }
}

/// Re-apply the layer's size, exclusive zone and regions when the settings change them (the
/// daemon spawns the dock once with the settings of that moment).
fn follow_layer(config: LayerConfig) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(config.clone()));
    if *last.peek() == config {
        return;
    }
    if let Some(surface) = surface {
        surface.set_size(config.size);
        surface.set_exclusive_zone(config.exclusive);
        surface.set_input_region(config.input.clone());
        surface.set_blur(config.blur.clone());
    }
    last.set(config);
}

/// `v` in logical pixels.
fn px(v: Cpx) -> f64 {
    f64::from(v.0) / 100.0
}

/// The layer that carries the surface's coordinates inside the pill-sized root: moved back by
/// the pill's corner, the output's width and the layer's height.
fn surface_style(pill: &Area, width: i32, height: Cpx) -> String {
    format!(
        "left:{:.2}px;top:{:.2}px;width:{width}px;height:{:.2}px",
        -px(pill.left),
        -px(pill.top),
        px(height)
    )
}

fn area_style(area: &Area) -> String {
    format!(
        "left:{:.2}px;top:{:.2}px;width:{:.2}px;height:{:.2}px",
        px(area.left),
        px(area.top),
        px(area.width),
        px(area.height)
    )
}

/// The separator: centred in its gap, vertically centred on the rest tiles.
fn separator_style(x: Cpx, rows: &Rows, height: u16) -> String {
    let middle = px(rows.pill_top) + px(rows.pill_height) / 2.0;
    format!(
        "left:{:.2}px;top:{:.2}px;height:{height}px",
        px(x) - 0.5,
        middle - f64::from(height) / 2.0
    )
}

/// The auto-hide trigger strip: the bottom `strip_px` of the output across the rest pill
/// (design/10 §10.3.11), moved up by the slide so it stays at the edge while hidden.
fn strip_area(rest: &Frame, rows: &Rows, strip_px: i32, slide: Cpx) -> Area {
    Area {
        left: rest.pill.left,
        top: Cpx(rows.surface.0 - Cpx::px(strip_px).0 - slide.0),
        width: rest.pill.width,
        height: Cpx::px(strip_px),
    }
}
