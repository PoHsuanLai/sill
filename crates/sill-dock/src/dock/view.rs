//! The dock's render: read the services and settings, hand the session this render's world,
//! lay the frame out from the machine's displayed progress and draw it (design/10 §10.3.1,
//! §10.3.12). The surface is transparent everywhere but the pill and the tiles. The pill is the
//! `Ds` root itself: a `Material::Dock` root in a box at the pill's rectangle, which draws the
//! Space gradient at the Dock's tint alpha with the material stack's hairline, highlight and
//! shadows (quire's tinted frame), its corner a squircle of `dock.pill_radius_px` (the macOS
//! polish pass). The box around the root carries the shell type scale and quire's dock metrics
//! (`DockMetrics`: gap, padding, the running dot), and the root's first child is quire's
//! `DockFloor` (off unless `dock.floor` is on). Everything else hangs from the root in a layer
//! moved back to the surface's origin, so tiles, dots and the hit elements keep surface
//! coordinates. The root's content is on paper (`Ground::Paper`): the tiles are quire plates,
//! neutral paper for third-party icons (design/08 §4.1), and the separator names the frame ink
//! itself.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::chrome::Ground;
use ds::style::appearance::blur::BlurState;
use shell_host::{LayerConfig, OutputInfo, SurfaceHandle};
use sill_model::ids::OutputName;
use sill_model::units::Cpx;
use sill_settings::AutoHide;
use sill_shell_kit::prelude::*;

use crate::dock::frame::{Area, Frame, Rows, frame};
use crate::dock::geometry::{dock_metrics, pill_corner};
use crate::dock::io::{SessionDeps, World, use_dock};
use crate::dock::items::dock_items;
use crate::dock::layer::layer;
use crate::dock::machine::{DockParams, DockState, Label, Reveal, displayed, lift, slide_offset};
use crate::dock::pointer::{DockInput, PointerInput, button, modifier, point};
use crate::dock::tile::{LabelShown, Tile, TileLook};
use sill_services::{AppsService, BadgesService, CosmicWlService, PinsService};
use sill_ui_kit::app_icons::icon_look;
use sill_ui_kit::hooks::{use_appearance, use_service, use_service_part, use_settings};
use sill_ui_kit::look::{active_workspace, use_active_look};
use sill_ui_kit::menu::PopupLook;
use sill_ui_kit::root::use_root_style;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::shell_style::shell_metrics;

/// The dock's own stylesheet (lint-clean under `Profile::Strict`, `tests/dock_lint.rs`).
pub const DOCK_CSS: &str = include_str!("style.css");

/// The dock on `output`.
pub(super) fn dock(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let root = use_root_style();
    let feed = use_context::<SettingsFeed>();
    let on = output.name.clone().map(OutputName);
    let look = use_active_look(&output);
    let workspace = active_workspace(
        &use_service_part::<CosmicWlService, _>(|wl| wl.workspaces.clone())(),
        on.as_ref(),
    )
    .map(|(key, _)| key);
    let appearance = environment.settings.appearance.appearance();
    let level = resolve(appearance, look.theme, environment.system).motion;
    let params = DockParams::new(&file.dock, level);
    let world = World {
        toplevels: use_service_part::<CosmicWlService, _>(|wl| wl.toplevels.clone())(),
        index: use_service::<AppsService>()(),
        pins: use_service::<PinsService>()(),
        badges: use_service::<BadgesService>()(),
        workspace,
    };
    let metrics = shell_metrics(&file);
    let stack = environment.material_stack();
    let items = dock_items(&world.pins, &world.toplevels, &world.index, file.dock.trash);
    let initial = DockState::new(match file.dock.autohide {
        AutoHide::Off => Reveal::Fixed,
        AutoHide::On => Reveal::Hidden,
    });
    let session = use_dock(initial);
    follow_layer(layer(&output, &file));
    let machine = session.machine.read().clone();
    let label = LabelShown(match machine.label {
        Label::Shown { tile } => Some(tile),
        Label::Idle | Label::Pending { .. } | Label::Warm { .. } => None,
    });
    let mut labels = use_context_provider(|| Signal::new(LabelShown(None)));
    if *labels.peek() != label {
        labels.set(label);
    }
    // The icon style, with the active Space's tint: a workspace switch that changes the tint
    // re-keys every re-tinted tile once, here, not per frame.
    let icons = TileLook(icon_look(
        &environment.settings.icons,
        &look,
        appearance.accent,
        resolve(appearance, look.theme, environment.system).scheme,
    ));
    let mut tile_look = use_context_provider(|| Signal::new(TileLook::default()));
    if *tile_look.peek() != icons {
        tile_look.set(icons);
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
    let rest = frame(
        &items,
        &file.dock,
        width,
        sill_model::units::Permille::ZERO,
        x,
    );
    session.update(SessionDeps {
        items: items.clone(),
        frame: shown.clone(),
        params,
        world: world.clone(),
        order: file.dock.context_menu_order,
        feed: feed.clone(),
        look: PopupLook {
            appearance,
            system: environment.system,
            tint: environment.tint_alpha(),
            metrics,
            stack,
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
            style: "{area_style(&shown.pill)};transform:translateY({px(slide)}px);{metrics.style_attr()}{dock_metrics(&file.dock).style_attr()}",
            Ds {
                sheet: Some(sill_shell_kit::stylesheet()),
                appearance,
                system: environment.system,
                look,
                material: Material::Dock,
                radius: Some(pill_corner(&file.dock)),
                ground: Some(Ground::Paper),
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                stack: Some(stack),
                surface: root.surface,
                user_style: root.user_style,
                AppStyle { css: DOCK_CSS }
                DockFloor {}
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
