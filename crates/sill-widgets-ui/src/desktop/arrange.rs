//! Moving one widget (a drag, macOS Sonoma's desktop widgets): where a widget dragged to a point
//! snaps, and how the others make room for it. Pure.
//!
//! Making room is worked out from the widgets' resting places every time, never from the last
//! arrangement: a widget pushed aside goes back to its own place as soon as the dragged one
//! moves on, as the reference's desktop does with what a dragged widget passes over.

use sill_settings::WidgetKind;

use super::geometry::GridMetrics;
use super::placement::{Footprint, GridSize, clamp, nearest_free};
use sill_ui_kit::widgets::grid::{GridSlot, Placement};
use sill_ui_kit::widgets::{WidgetSize, desktop_size};

/// A point on the layer in whole logical px (may lie off it: a drag goes where the pointer goes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerPoint {
    pub x: i32,
    pub y: i32,
}

impl LayerPoint {
    /// `self - other`, per axis.
    pub fn minus(self, other: LayerPoint) -> LayerPoint {
        LayerPoint {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    /// `|dx| + |dy|` to `other`.
    pub fn manhattan(self, other: LayerPoint) -> u32 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

/// The cell a `size`d widget whose top-left corner is at `corner` snaps to: the nearest cell,
/// clamped so the widget fits the grid; none when it fits nowhere.
pub fn snap(corner: LayerPoint, size: WidgetSize, metrics: GridMetrics) -> Option<GridSlot> {
    let cell = i64::from(metrics.cell);
    let gap = i64::from(metrics.gap);
    let step = cell + gap;
    let used = i64::from(metrics.grid.columns) * step + gap;
    let origin = i64::from(metrics.layer_width) - used;
    let nearest = |from: i64| {
        let index = (from - gap).max(0) as f64 / step.max(1) as f64;
        u16::try_from(index.round() as i64).unwrap_or(u16::MAX)
    };
    let slot = GridSlot {
        column: nearest(i64::from(corner.x) - origin),
        row: nearest(i64::from(corner.y)),
    };
    clamp(slot, size, metrics.grid)
}

/// The arrangement with `moving` at `to` in `grid`: every other widget keeps its resting place
/// when that is clear of the moved one, and each one it covers takes the free spot nearest its
/// resting place (in their order in `rest`). None when one of them fits nowhere (the drag then
/// has no room there, and a drop there puts the widget back). Each widget is its one desktop
/// size.
pub fn make_room(
    rest: &[Placement],
    moving: WidgetKind,
    to: GridSlot,
    grid: GridSize,
) -> Option<Vec<Placement>> {
    let size = desktop_size;
    let moved = Footprint::new(to, size(moving));
    let others: Vec<&Placement> = rest.iter().filter(|p| p.kind != moving).collect();
    let (stay, pushed): (Vec<&Placement>, Vec<&Placement>) = others
        .iter()
        .partition(|p| Footprint::new(p.slot, size(p.kind)).free_in(grid, &[moved]));
    let mut taken: Vec<Footprint> = std::iter::once(moved)
        .chain(stay.iter().map(|p| Footprint::new(p.slot, size(p.kind))))
        .collect();
    let mut moves: Vec<Placement> = Vec::new();
    for p in pushed {
        let spot = nearest_free(size(p.kind), p.slot, grid, &taken)?;
        taken.push(spot);
        moves.push(Placement {
            kind: p.kind,
            slot: spot.slot,
        });
    }
    Some(
        rest.iter()
            .map(|p| match p.kind == moving {
                true => Placement {
                    kind: moving,
                    slot: to,
                },
                false => moves
                    .iter()
                    .find(|m| m.kind == p.kind)
                    .copied()
                    .unwrap_or(*p),
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "arrange_tests.rs"]
mod tests;
