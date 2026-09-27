//! A widget's drag on the desktop layer (macOS Sonoma: press a widget and move it; it lifts,
//! the others make room, and a release snaps it into the grid), as a pure machine over the
//! layer's own pointer input. A press is nothing yet: only travel past the threshold
//! (`window.move_threshold_px`) makes it a drag, so a click or a long press still reaches the
//! widget's own controls (Now Playing's buttons, the calendar's arrows).

use sill_settings::WidgetKind;

use super::arrange::{LayerPoint, make_room, snap};
use super::geometry::{GridMetrics, PlaceBox};
use super::placement::{GridSlot, Placement};
use crate::widgets::desktop_size;

/// One pointer input on the layer, in layer px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragInput {
    Down(LayerPoint),
    Move(LayerPoint),
    Up(LayerPoint),
}

/// Where the primary button is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hold {
    #[default]
    Idle,
    /// Down on `kind` at `from`, `grab` from the widget's top-left corner: not a drag yet.
    Pressed {
        kind: WidgetKind,
        from: LayerPoint,
        grab: LayerPoint,
    },
    /// Dragging `kind`: the pointer at `at`, the widget's corner at `at - grab`, and the cell it
    /// would snap to (none when it fits nowhere or there is no room there).
    Live {
        kind: WidgetKind,
        at: LayerPoint,
        grab: LayerPoint,
        over: Option<GridSlot>,
    },
}

impl Hold {
    /// The dragged widget's top-left corner while live.
    pub fn lifted(&self) -> Option<(WidgetKind, LayerPoint)> {
        match *self {
            Hold::Live { kind, at, grab, .. } => Some((kind, at.minus(grab))),
            Hold::Idle | Hold::Pressed { .. } => None,
        }
    }
}

/// What a step decided beyond the next hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landing {
    /// Nothing to keep.
    Nothing,
    /// A drag ended: this is the new arrangement (the old one when there was no room).
    Dropped {
        kind: WidgetKind,
        layout: Vec<Placement>,
    },
}

/// What a step reads: the resting arrangement, where each widget is drawn, the grid and the
/// travel that makes a press a drag.
#[derive(Debug, Clone, Copy)]
pub struct DragWorld<'a> {
    pub rest: &'a [Placement],
    pub boxes: &'a [(WidgetKind, PlaceBox)],
    pub metrics: GridMetrics,
    pub threshold: u32,
}

/// The next hold after `input`, and what it lands.
pub fn step(hold: Hold, input: DragInput, world: DragWorld<'_>) -> (Hold, Landing) {
    match (hold, input) {
        (Hold::Idle, DragInput::Down(at)) => (pressed(at, world.boxes), Landing::Nothing),
        (Hold::Pressed { kind, from, grab }, DragInput::Move(at))
            if from.manhattan(at) >= world.threshold =>
        {
            (live(kind, at, grab, world), Landing::Nothing)
        }
        (Hold::Live { kind, grab, .. }, DragInput::Move(at)) => {
            (live(kind, at, grab, world), Landing::Nothing)
        }
        (Hold::Live { kind, grab, .. }, DragInput::Up(at)) => {
            let layout = match live(kind, at, grab, world) {
                Hold::Live { over: Some(to), .. } => {
                    make_room(world.rest, kind, to, world.metrics.grid)
                }
                Hold::Live { over: None, .. } | Hold::Idle | Hold::Pressed { .. } => None,
            };
            let layout = layout.unwrap_or_else(|| world.rest.to_vec());
            (Hold::Idle, Landing::Dropped { kind, layout })
        }
        (Hold::Pressed { .. }, DragInput::Up(_)) => (Hold::Idle, Landing::Nothing),
        (Hold::Pressed { .. } | Hold::Live { .. }, DragInput::Down(_))
        | (Hold::Pressed { .. }, DragInput::Move(_))
        | (Hold::Idle, DragInput::Move(_) | DragInput::Up(_)) => (hold, Landing::Nothing),
    }
}

/// A press at `at`: on a widget, held; between widgets, nothing.
fn pressed(at: LayerPoint, boxes: &[(WidgetKind, PlaceBox)]) -> Hold {
    let hit = boxes.iter().find(|(_, b)| contains(b, at));
    match hit {
        Some(&(kind, b)) => Hold::Pressed {
            kind,
            from: at,
            grab: at.minus(corner(&b)),
        },
        None => Hold::Idle,
    }
}

/// Dragging `kind` with the pointer at `at`: the cell it snaps to, when there is room there.
fn live(kind: WidgetKind, at: LayerPoint, grab: LayerPoint, world: DragWorld<'_>) -> Hold {
    let over = snap(at.minus(grab), desktop_size(kind), world.metrics)
        .filter(|to| make_room(world.rest, kind, *to, world.metrics.grid).is_some());
    Hold::Live {
        kind,
        at,
        grab,
        over,
    }
}

/// The arrangement to draw under `hold`: the others making room round a live drag.
pub fn shown(hold: &Hold, rest: &[Placement], metrics: GridMetrics) -> Vec<Placement> {
    match *hold {
        Hold::Live {
            kind,
            over: Some(to),
            ..
        } => make_room(rest, kind, to, metrics.grid).unwrap_or_else(|| rest.to_vec()),
        Hold::Live { over: None, .. } | Hold::Idle | Hold::Pressed { .. } => rest.to_vec(),
    }
}

/// A box's top-left corner.
pub fn corner(b: &PlaceBox) -> LayerPoint {
    let px = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    LayerPoint {
        x: px(b.left),
        y: px(b.top),
    }
}

fn contains(b: &PlaceBox, at: LayerPoint) -> bool {
    let c = corner(b);
    let px = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    at.x >= c.x && at.y >= c.y && at.x < c.x + px(b.width) && at.y < c.y + px(b.height)
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
