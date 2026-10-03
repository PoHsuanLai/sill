//! The effects end of the desktop layer: read the places again when Edit Widgets changes the
//! desktop's widgets, turn pointer events into drag inputs and carry a drop's places to the
//! file, and keep the surface's blur region on the placed frames.

use dioxus::prelude::*;
use ds::motion::detail::touch::Touch;
use ds::motion::spring_point::Release;
use shell_host::SurfaceHandle;
use sill_settings::WidgetKind;

use super::arrange::LayerPoint;
use super::drag::{DragInput, DragWorld, Hold, Landing, step};
use super::glide::PointMeter;
use super::model::{Dropped, Laid, merged, widget_blur};
use sill_ui_kit::links::{PlacementFile, Written};
use sill_ui_kit::widgets::grid::{OutputKey, Placement};

/// Read this output's places from the file again when its widgets change or Edit Widgets has
/// written (it wrote them there), so a widget it added lands in the cell it chose.
pub(super) fn use_reload(
    (kinds, written): (&[WidgetKind], Written),
    file: Option<&PlacementFile>,
    key: &OutputKey,
    mut kept: CopyValue<Vec<Placement>>,
) {
    let now = (kinds.to_vec(), written);
    let mut seen = use_hook(|| CopyValue::new(now.clone()));
    if *seen.peek() == now {
        return;
    }
    seen.set(now);
    if let Some(file) = file {
        kept.set(file.load(key));
    }
}

/// Everything a pointer input needs, cheap to clone into each handler.
#[derive(Clone)]
pub(super) struct Feeder {
    pub(super) hold: Signal<Hold>,
    pub(super) dropped: Signal<Option<Dropped>>,
    pub(super) meter: CopyValue<PointMeter>,
    pub(super) laid: CopyValue<Option<Laid>>,
    pub(super) kept: CopyValue<Vec<Placement>>,
    pub(super) file: Option<PlacementFile>,
    pub(super) key: OutputKey,
}

impl Feeder {
    /// A primary-button pointer event on the grid.
    pub(super) fn pointer(&self, event: &Event<MouseData>, input: fn(LayerPoint) -> DragInput) {
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
        let when = ds::base::time::clock::now();
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

/// A press of the secondary button on the layer opens the desktop's menu with its corner there;
/// any other button does nothing here.
pub(super) fn secondary_press(event: &Event<MouseData>, open: Callback<LayerPoint>) {
    use dioxus::html::input_data::MouseButton;
    use dioxus::html::point_interaction::{InteractionLocation, PointerInteraction};
    if event.trigger_button() == Some(MouseButton::Secondary) {
        let at = event.client_coordinates();
        open.call(LayerPoint {
            x: at.x.round() as i32,
            y: at.y.round() as i32,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Primary {
    Yes,
    No,
}

pub(super) fn save(file: Option<&PlacementFile>, key: &OutputKey, placements: &[Placement]) {
    if let Some(file) = file
        && let Err(error) = file.save(key, placements)
    {
        eprintln!("sill: saving {}: {error}", file.0.display());
    }
}

/// Keep the surface's blur region on the placed frames, sending it only when the set of widgets
/// changes (the region is the frames' elements, resolved after each layout, so it follows a
/// frame that moves). Nothing to do without a host (a test, an SSR render).
pub(super) fn use_regions(placements: &[Placement]) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<Vec<WidgetKind>>));
    let kinds: Vec<WidgetKind> = placements.iter().map(|p| p.kind).collect();
    if last.peek().as_deref() == Some(kinds.as_slice()) {
        return;
    }
    if let Some(surface) = surface {
        surface.set_blur(widget_blur(placements));
    }
    last.set(Some(kinds));
}
