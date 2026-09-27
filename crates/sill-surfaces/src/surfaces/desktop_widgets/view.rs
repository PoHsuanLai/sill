//! The layer's root: a transparent root in the Widget material as large as the layer
//! (`RootExtent::Viewport`), holding the grid. The grid measures itself once laid out, places
//! `widgets.desktop_widgets` with `place` over `grid_size` (this output's saved places first,
//! from the [`PlacementFile`] when the root was given one), draws each widget at its box, lets a
//! widget be dragged to another cell (`drag`, `placed`), and keeps the surface's input and blur
//! regions on the frames (`RegionSpec::Elements`, resolved after every layout, so they follow a
//! dragged frame), so a press between them reaches the desktop below. Nothing here runs a timer:
//! the widgets redraw on their services' changes, and a drag's motion runs only while it moves.

use dioxus::prelude::*;
use ds::detail::{Ease, Touch};
use ds::{BlurState, Ds, Material, Px, RootChrome, RootExtent, WakeStamp, WidgetMetrics, use_rect};
use shell_host::{BlurSpec, ElementId, ElementRegion, OutputInfo, RegionSpec, SurfaceHandle};
use sill_services::OutputName;
use sill_settings::WidgetKind;

use super::DESKTOP_WIDGETS_CSS;
use super::arrange::LayerPoint;
use super::drag::{DragInput, DragWorld, Hold, Landing, shown, step};
use super::geometry::{GridMetrics, PlaceBox, place_box};
use super::placed::{Placed, Pose};
use super::placement::{Placement, grid_size, place};
use super::saved::{OutputKey, PlacementFile};
use crate::hooks::{use_appearance, use_ds_scale, use_settings, use_workspaces};
use crate::surfaces::launcher::panel::active_look;
use crate::widgets::{WidgetHost, provide_widget_wake, size_of, widget_element_id, widgets_css};

/// The Widget material's corner (quire's recipe, `--m-radius` 20 px): the frames' regions round
/// their corners by it.
pub const WIDGET_RADIUS: i32 = 20;

/// The layer on `output`: its own widget wake (`crate::widgets::WidgetWake`), bumped once when
/// this layer is mapped (`widgets.desktop` turning `On` spawns a fresh one, FINDINGS "M10 freeze"
/// `crates/sill/src/widgets.rs`), so the batteries' rings and figures count up as they come into
/// view.
pub(super) fn desktop(output: OutputInfo) -> Element {
    provide_widget_wake(WakeStamp::default());
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let widgets = file.widgets;
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Widget,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {DESKTOP_WIDGETS_CSS} }
            style { {widgets_css()} }
            Grid {
                kinds: widgets.desktop_widgets,
                cell: widgets.desktop_cell_px.0,
                gap: widgets.desktop_gap_px.0,
                threshold: file.window.move_threshold_px.0,
                output,
            }
        }
    }
}

/// What the grid last laid out: what a pointer input is resolved against.
#[derive(Debug, Clone, PartialEq)]
struct Laid {
    rest: Vec<Placement>,
    boxes: Vec<(WidgetKind, PlaceBox)>,
    metrics: GridMetrics,
    threshold: u32,
}

/// The last widget dropped, and who dropped it: it glides into its cell on a spring when the
/// person's hand did, and is drawn above the others until the next drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dropped {
    kind: WidgetKind,
    touch: Touch,
}

/// The widgets on their grid, once the layer's size is known.
#[component]
fn Grid(
    kinds: Vec<WidgetKind>,
    cell: u16,
    gap: u16,
    threshold: u16,
    output: OutputInfo,
) -> Element {
    let probe = use_rect();
    let file = try_use_context::<PlacementFile>();
    let key = OutputKey::of(output.name.as_deref());
    let mut kept =
        use_hook(|| CopyValue::new(file.as_ref().map(|f| f.load(&key)).unwrap_or_default()));
    let hold = use_signal(Hold::default);
    let dropped = use_signal(|| None::<Dropped>);
    let mut laid = use_hook(|| CopyValue::new(None::<Laid>));
    let now = probe.rect().map(|rect| {
        let width = logical(rect.size.width.0);
        let grid = grid_size(width, logical(rect.size.height.0), cell, gap);
        let sized: Vec<_> = kinds
            .iter()
            .map(|kind| (*kind, size_of(*kind, WidgetHost::Desktop)))
            .collect();
        let metrics = GridMetrics {
            grid,
            layer_width: width,
            cell,
            gap,
        };
        let rest = place(&sized, &kept.peek(), grid);
        Laid {
            boxes: boxes(&rest, metrics),
            rest,
            metrics,
            threshold: u32::from(threshold),
        }
    });
    if let Some(now) = &now {
        let recorded = recorded(&now.rest, &kept.peek());
        if recorded != *kept.peek() {
            save(file.as_ref(), &key, &recorded);
            kept.set(recorded);
        }
    }
    laid.set(now.clone());
    let feed = Feeder {
        hold,
        dropped,
        laid,
        kept,
        file: file.clone(),
        key: key.clone(),
    };
    let held = hold();
    let on_screen = now
        .as_ref()
        .map(|l| shown(&held, &l.rest, l.metrics))
        .unwrap_or_default();
    use_regions(&on_screen);
    let places = now
        .map(|l| draw_order(&on_screen, &held, dropped(), l.metrics))
        .unwrap_or_default();
    let (down, moved, up) = (feed.clone(), feed.clone(), feed);
    rsx! {
        div {
            class: "sill-dw-grid",
            style: grid_style(cell, gap),
            onmounted: move |event| probe.on_mounted(event),
            onmousedown: move |event| down.pointer(&event, DragInput::Down),
            onmousemove: move |event| moved.pointer(&event, DragInput::Move),
            onmouseup: move |event| up.pointer(&event, DragInput::Up),
            for (kind, target, pose, ease) in places {
                Placed { key: "{kind:?}", kind, target, pose, ease }
            }
        }
    }
}

/// Each widget's box at its resting cell.
fn boxes(placements: &[Placement], metrics: GridMetrics) -> Vec<(WidgetKind, PlaceBox)> {
    placements
        .iter()
        .map(|p| {
            let size = size_of(p.kind, WidgetHost::Desktop);
            (p.kind, place_box(p.slot, size, metrics))
        })
        .collect()
}

/// What each widget draws, in paint order: the dragged one (else the last dropped) last, so it
/// is above the others.
fn draw_order(
    on_screen: &[Placement],
    held: &Hold,
    dropped: Option<Dropped>,
    metrics: GridMetrics,
) -> Vec<(WidgetKind, PlaceBox, Pose, Ease)> {
    let lifted = held.lifted();
    let top = lifted.map(|(kind, _)| kind).or(dropped.map(|d| d.kind));
    let mut places: Vec<_> = boxes(on_screen, metrics)
        .into_iter()
        .map(|(kind, target)| {
            let pose = match lifted {
                Some((held, corner)) if held == kind => Pose::Lifted(corner),
                Some(_) | None => Pose::Resting,
            };
            let ease = match dropped {
                Some(Dropped {
                    kind: last,
                    touch: Touch::Contact(contact),
                }) if last == kind => Ease::Spring(contact),
                Some(_) | None => Ease::Out,
            };
            (kind, target, pose, ease)
        })
        .collect();
    places.sort_by_key(|(kind, ..)| Some(*kind) == top);
    places
}

/// Everything a pointer input needs, cheap to clone into each handler.
#[derive(Clone)]
struct Feeder {
    hold: Signal<Hold>,
    dropped: Signal<Option<Dropped>>,
    laid: CopyValue<Option<Laid>>,
    kept: CopyValue<Vec<Placement>>,
    file: Option<PlacementFile>,
    key: OutputKey,
}

impl Feeder {
    /// A primary-button pointer event on the grid.
    fn pointer(&self, event: &Event<MouseData>, input: fn(LayerPoint) -> DragInput) {
        use dioxus::html::input_data::MouseButton;
        use dioxus::html::point_interaction::{InteractionLocation, PointerInteraction};
        let primary = match event.trigger_button() {
            Some(MouseButton::Primary) | None => Primary::Yes,
            Some(_) => Primary::No,
        };
        if primary == Primary::No {
            return;
        }
        let at = event.client_coordinates();
        let point = LayerPoint {
            x: at.x.round() as i32,
            y: at.y.round() as i32,
        };
        self.feed(input(point), Touch::from_event(event));
    }

    /// Step the drag with `input`; a drop becomes this output's saved places.
    fn feed(&self, input: DragInput, touch: Touch) {
        let Some(laid) = self.laid.peek().clone() else {
            return;
        };
        let world = DragWorld {
            rest: &laid.rest,
            boxes: &laid.boxes,
            metrics: laid.metrics,
            threshold: laid.threshold,
        };
        let now = *self.hold.peek();
        let (next, landing) = step(now, input, world);
        let mut hold = self.hold;
        if next != now {
            hold.set(next);
        }
        match landing {
            Landing::Nothing => {}
            Landing::Dropped { kind, layout } => {
                let saved = merged(&layout, &self.kept.peek());
                save(self.file.as_ref(), &self.key, &saved);
                let mut kept = self.kept;
                kept.set(saved);
                let mut dropped = self.dropped;
                dropped.set(Some(Dropped { kind, touch }));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Primary {
    Yes,
    No,
}

fn save(file: Option<&PlacementFile>, key: &OutputKey, placements: &[Placement]) {
    if let Some(file) = file
        && let Err(error) = file.save(key, placements)
    {
        eprintln!("sill: saving {}: {error}", file.0.display());
    }
}

/// The grid unit as quire's widget tokens (`--widget-cell`, `--widget-gap`), so each
/// `WidgetFrame` is as large as the place the layer gives it.
fn grid_style(cell: u16, gap: u16) -> String {
    WidgetMetrics {
        cell: Px(f32::from(cell)),
        gap: Px(f32::from(gap)),
    }
    .style_attr()
}

/// A measured length in whole logical px.
fn logical(length: f32) -> u32 {
    // Clamped into u32's range first, so the cast only drops the fraction.
    length.round().clamp(0.0, 1.0e9) as u32
}

/// What the file keeps after a layout: every saved place as it was (a place clamped onto a
/// smaller output is not written over, so it comes back when the output does), then the places
/// of widgets that had none.
fn recorded(placed: &[Placement], saved: &[Placement]) -> Vec<Placement> {
    let first_seen = placed
        .iter()
        .filter(|p| saved.iter().all(|s| s.kind != p.kind));
    placed
        .iter()
        .filter_map(|p| saved.iter().find(|s| s.kind == p.kind))
        .chain(
            saved
                .iter()
                .filter(|s| placed.iter().all(|p| p.kind != s.kind)),
        )
        .chain(first_seen)
        .copied()
        .collect()
}

/// A drop's places: this layer's, then every saved one of a kind it does not show (a widget
/// taken off the desktop keeps its place for when it comes back).
fn merged(placed: &[Placement], saved: &[Placement]) -> Vec<Placement> {
    let others = saved
        .iter()
        .filter(|s| placed.iter().all(|p| p.kind != s.kind));
    placed.iter().chain(others).copied().collect()
}

/// The input and blur regions `placements` take: each widget's frame.
pub fn widget_regions(placements: &[Placement]) -> (RegionSpec, BlurSpec) {
    if placements.is_empty() {
        return (RegionSpec::Empty, BlurSpec::None);
    }
    let regions: Vec<ElementRegion> = placements
        .iter()
        .map(|p| ElementRegion {
            id: ElementId::new(widget_element_id(p.kind)),
            radius: WIDGET_RADIUS,
            inflate: 0,
        })
        .collect();
    (
        RegionSpec::Elements(regions.clone()),
        BlurSpec::Region(RegionSpec::Elements(regions)),
    )
}

/// Keep the surface's regions on the placed frames, sending them only when the set of widgets
/// changes (the regions are the frames' elements, resolved after each layout, so they follow a
/// frame that moves). Nothing to do without a host (a test, an SSR render).
fn use_regions(placements: &[Placement]) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<Vec<WidgetKind>>));
    let kinds: Vec<WidgetKind> = placements.iter().map(|p| p.kind).collect();
    if last.peek().as_deref() == Some(kinds.as_slice()) {
        return;
    }
    if let Some(surface) = surface {
        let (input, blur) = widget_regions(placements);
        surface.set_input_region(input);
        surface.set_blur(blur);
    }
    last.set(Some(kinds));
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
