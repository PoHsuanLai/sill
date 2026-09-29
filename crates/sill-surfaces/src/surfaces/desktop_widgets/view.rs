//! The layer's root: a transparent root in the Widget material as large as the layer
//! (`RootExtent::Viewport`), holding the grid. The grid measures itself once laid out, places
//! `widgets.desktop_widgets` (each at its one desktop size) with `place` over `grid_size`
//! (this output's saved places first, from the [`PlacementFile`] when the root was given one,
//! read again when Edit Widgets changes the desktop's widgets), draws each widget at its box,
//! lets a widget be dragged to another cell (`drag`, `placed`; quire's `WidgetSlotGuide` at the
//! cell it would land in), and keeps the surface's input and blur
//! regions on the frames (`RegionSpec::Elements`, resolved after every layout, so they follow a
//! dragged frame), so a press between them reaches the desktop below. A widget taken away keeps
//! its card at its last box while it plays its exit, outside the regions (`leaving`).
//! Nothing here runs a timer: the widgets redraw on their services' changes, a drag's motion runs
//! only while it moves, and an exit's end is quire's card's to call.

use dioxus::prelude::*;
use ds::detail::Touch;
use ds::motion::Release;
use ds::{
    BlurState, Ds, Material, Px, RootChrome, RootExtent, WakeStamp, WidgetMetrics, WidgetSlotGuide,
    use_rect,
};
use shell_host::{BlurSpec, ElementId, ElementRegion, OutputInfo, RegionSpec, SurfaceHandle};
use sill_settings::WidgetKind;

use super::DESKTOP_WIDGETS_CSS;
use super::arrange::LayerPoint;
use super::drag::{DragInput, DragWorld, Hold, Landing, shown, step};
use super::geometry::{GridMetrics, PlaceBox, place_box};
use super::glide::{Glide, PointMeter, Pose};
use super::leaving::{Leaving, use_leaving};
use super::placed::Placed;
use super::placement::{Placement, grid_size, place};
use super::saved::{OutputKey, PlacementFile};
use crate::hooks::{use_appearance, use_ds_scale, use_settings};
use crate::look::use_active_look;
use crate::widgets::{
    CardPresence, WidgetHost, WidgetSize, desktop_size, provide_registry, provide_widget_wake,
    widget_element_id, widgets_css,
};

/// The Widget material's corner (quire's recipe, `--m-radius` 20 px): the frames' regions round
/// their corners by it.
pub const WIDGET_RADIUS: i32 = 20;

/// The layer on `output`: its own widget wake (`crate::widgets::WidgetWake`), bumped once when
/// this layer is mapped (`widgets.desktop` turning `On` spawns a fresh one,
/// `crates/sill/src/widgets.rs`), so the batteries' rings and figures count up as they come into
/// view.
pub(super) fn desktop(output: OutputInfo) -> Element {
    provide_widget_wake(WakeStamp::default());
    provide_registry();
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let look = use_active_look(&output);
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

/// The last widget dropped, the contact that let it go and the hand's velocity then: it springs
/// into its cell from there, and is drawn above the others until the next drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dropped {
    kind: WidgetKind,
    touch: Touch,
    release: Release,
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
    use_reload(&kinds, file.as_ref(), &key, kept);
    let hold = use_signal(Hold::default);
    let dropped = use_signal(|| None::<Dropped>);
    let meter = use_hook(|| CopyValue::new(PointMeter::default()));
    let mut laid = use_hook(|| CopyValue::new(None::<Laid>));
    let cards = use_leaving(&kinds);
    let now = probe.rect().map(|rect| {
        let width = logical(rect.size.width.0);
        let grid = grid_size(width, logical(rect.size.height.0), cell, gap);
        let sized: Vec<_> = kinds
            .iter()
            .map(|kind| (*kind, desktop_size(*kind)))
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
    // Where each card stood before this layout: a kind just taken away leaves from there.
    let last_boxes = laid.peek().as_ref().map(|l| l.boxes.clone());
    let leaving = cards.now(&kinds, &last_boxes.unwrap_or_default());
    laid.set(now.clone());
    let feed = Feeder {
        hold,
        dropped,
        meter,
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
    let guide = now.as_ref().and_then(|l| slot_guide(&held, l));
    let places = now
        .map(|l| draw_order(&leaving, &on_screen, &held, dropped(), l.metrics))
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
            if let Some((size, spot)) = guide {
                div { class: "sill-dw-guide", style: spot.style_attr(),
                    WidgetSlotGuide { size, host: WidgetHost::Desktop }
                }
            }
            for Draw { kind, target, pose, glide, presence } in places {
                Placed {
                    key: "{kind:?}",
                    kind,
                    target,
                    pose,
                    glide,
                    presence,
                    on_gone: match presence {
                        CardPresence::Leaving => Some(cards.on_gone(kind)),
                        CardPresence::Placed => None,
                    },
                }
            }
        }
    }
}

/// Each widget's box at its resting cell.
fn boxes(placements: &[Placement], metrics: GridMetrics) -> Vec<(WidgetKind, PlaceBox)> {
    placements
        .iter()
        .map(|p| (p.kind, place_box(p.slot, desktop_size(p.kind), metrics)))
        .collect()
}

/// Where a live drag would land, as the slot guide's size and box: none at rest, while
/// pressed, or over a cell with no room.
fn slot_guide(held: &Hold, laid: &Laid) -> Option<(WidgetSize, PlaceBox)> {
    let Hold::Live {
        kind,
        over: Some(slot),
        ..
    } = *held
    else {
        return None;
    };
    let size = desktop_size(kind);
    Some((size, place_box(slot, size, laid.metrics)))
}

/// Read this output's places from the file again when the desktop's widgets change (Edit
/// Widgets wrote them there), so a widget it added lands in the cell it chose.
fn use_reload(
    kinds: &[WidgetKind],
    file: Option<&PlacementFile>,
    key: &OutputKey,
    mut kept: CopyValue<Vec<Placement>>,
) {
    let now = kinds.to_vec();
    let mut seen = use_hook(|| CopyValue::new(now.clone()));
    if *seen.peek() == now {
        return;
    }
    seen.set(now);
    if let Some(file) = file {
        kept.set(file.load(key));
    }
}

/// One card the grid draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Draw {
    kind: WidgetKind,
    target: PlaceBox,
    pose: Pose,
    glide: Glide,
    presence: CardPresence,
}

/// What each widget draws, in paint order: the cards leaving first, under the rest, then the
/// placed ones with the dragged one (else the last dropped) last, so it is above the others.
fn draw_order(
    leaving: &[Leaving],
    on_screen: &[Placement],
    held: &Hold,
    dropped: Option<Dropped>,
    metrics: GridMetrics,
) -> Vec<Draw> {
    let lifted = held.lifted();
    let top = lifted.map(|(kind, _)| kind).or(dropped.map(|d| d.kind));
    let going = leaving.iter().map(|l| Draw {
        kind: l.kind,
        target: l.target,
        pose: Pose::Resting,
        glide: Glide::Aside,
        presence: CardPresence::Leaving,
    });
    let mut places: Vec<Draw> = boxes(on_screen, metrics)
        .into_iter()
        .map(|(kind, target)| {
            let pose = match lifted {
                Some((held, corner)) if held == kind => Pose::Lifted(corner),
                Some(_) | None => Pose::Resting,
            };
            let glide = match dropped {
                Some(Dropped {
                    kind: last,
                    touch,
                    release,
                }) if last == kind => Glide::Thrown { touch, release },
                Some(_) | None => Glide::Aside,
            };
            Draw {
                kind,
                target,
                pose,
                glide,
                presence: CardPresence::Placed,
            }
        })
        .collect();
    places.sort_by_key(|draw| Some(draw.kind) == top);
    going.chain(places).collect()
}

/// Everything a pointer input needs, cheap to clone into each handler.
#[derive(Clone)]
struct Feeder {
    hold: Signal<Hold>,
    dropped: Signal<Option<Dropped>>,
    meter: CopyValue<PointMeter>,
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
        let input = input(point);
        let release = self.measure(input);
        self.feed(input, Touch::from_event(event), release);
    }

    /// Keep the pointer's moves for the release velocity; a release reads it.
    fn measure(&self, input: DragInput) -> Release {
        let when = ds::time::now();
        let now = *self.meter.peek();
        let (next, release) = match input {
            DragInput::Down(at) => (PointMeter::pressed(at, when), Release::default()),
            DragInput::Move(at) => (now.moved(at, when), Release::default()),
            DragInput::Up(at) => (PointMeter::default(), now.released(at, when)),
        };
        let mut meter = self.meter;
        meter.set(next);
        release
    }

    /// Step the drag with `input`; a drop becomes this output's saved places, and the dropped
    /// widget springs from the hand at `release`.
    fn feed(&self, input: DragInput, touch: Touch, release: Release) {
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
                dropped.set(Some(Dropped {
                    kind,
                    touch,
                    release,
                }));
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
