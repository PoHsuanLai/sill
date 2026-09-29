//! What the desktop layer computes from its places: the last layout, the last drop, each card's
//! box and paint order, the places a layout or a drop keeps, and the regions the frames take.

use ds::CardPresence;
use ds::detail::Touch;
use ds::motion::Release;
use shell_host::{BlurSpec, ElementId, ElementRegion, RegionSpec};
use sill_settings::WidgetKind;

use super::drag::Hold;
use super::geometry::{GridMetrics, PlaceBox, place_box};
use super::glide::{Glide, Pose};
use super::leaving::Leaving;
use crate::widgets::grid::Placement;
use crate::widgets::{WidgetSize, desktop_size, widget_element_id};

/// What the grid last laid out: what a pointer input is resolved against.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Laid {
    pub(super) rest: Vec<Placement>,
    pub(super) boxes: Vec<(WidgetKind, PlaceBox)>,
    pub(super) metrics: GridMetrics,
    pub(super) threshold: u32,
}

/// The last widget dropped, the contact that let it go and the hand's velocity then: it springs
/// into its cell from there, and is drawn above the others until the next drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Dropped {
    pub(super) kind: WidgetKind,
    pub(super) touch: Touch,
    pub(super) release: Release,
}

/// Each widget's box at its resting cell.
pub(super) fn boxes(placements: &[Placement], metrics: GridMetrics) -> Vec<(WidgetKind, PlaceBox)> {
    placements
        .iter()
        .map(|p| (p.kind, place_box(p.slot, desktop_size(p.kind), metrics)))
        .collect()
}

/// Where a live drag would land, as the slot guide's size and box: none at rest, while
/// pressed, or over a cell with no room.
pub(super) fn slot_guide(held: &Hold, laid: &Laid) -> Option<(WidgetSize, PlaceBox)> {
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

/// One card the grid draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Draw {
    pub(super) kind: WidgetKind,
    pub(super) target: PlaceBox,
    pub(super) pose: Pose,
    pub(super) glide: Glide,
    pub(super) presence: CardPresence,
}

/// What each widget draws, in paint order: the cards leaving first, under the rest, then the
/// placed ones with the dragged one (else the last dropped) last, so it is above the others.
pub(super) fn draw_order(
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

/// A measured length in whole logical px.
pub(super) fn logical(length: f32) -> u32 {
    // Clamped into u32's range first, so the cast only drops the fraction.
    length.round().clamp(0.0, 1.0e9) as u32
}

/// What the file keeps after a layout: every saved place as it was (a place clamped onto a
/// smaller output is not written over, so it comes back when the output does), then the places
/// of widgets that had none.
pub(super) fn recorded(placed: &[Placement], saved: &[Placement]) -> Vec<Placement> {
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
pub(super) fn merged(placed: &[Placement], saved: &[Placement]) -> Vec<Placement> {
    let others = saved
        .iter()
        .filter(|s| placed.iter().all(|p| p.kind != s.kind));
    placed.iter().chain(others).copied().collect()
}

/// The Widget material's corner (quire's recipe, `--m-radius` 20 px): the frames' regions round
/// their corners by it.
pub const WIDGET_RADIUS: i32 = 20;

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
