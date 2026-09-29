use sill_settings::WidgetKind;

use super::{LayerPoint, make_room, snap};
use crate::surfaces::desktop_widgets::geometry::GridMetrics;
use crate::surfaces::desktop_widgets::placement::{GridSize, GridSlot, Placement};
use crate::widgets::WidgetSize;

/// 1000 px wide, five 164 px cells and 16 px gaps: column `c` starts at 100 + 180 c, row `r` at
/// 16 + 180 r (`geometry`'s own table).
pub(super) fn metrics() -> GridMetrics {
    GridMetrics {
        grid: GridSize {
            columns: 5,
            rows: 3,
        },
        layer_width: 1000,
        cell: 164,
        gap: 16,
    }
}

fn slot(column: u16, row: u16) -> GridSlot {
    GridSlot { column, row }
}

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: slot(column, row),
    }
}

fn point(x: i32, y: i32) -> LayerPoint {
    LayerPoint { x, y }
}

#[test]
fn a_dragged_corner_snaps_to_the_nearest_cell_that_fits() {
    let cases = [
        (
            "on a cell",
            point(280, 196),
            WidgetSize::Small,
            Some(slot(1, 1)),
        ),
        (
            "just short of the next",
            point(365, 100),
            WidgetSize::Small,
            Some(slot(1, 0)),
        ),
        (
            "just past half",
            point(375, 110),
            WidgetSize::Small,
            Some(slot(2, 1)),
        ),
        (
            "off the left",
            point(-400, 16),
            WidgetSize::Small,
            Some(slot(0, 0)),
        ),
        (
            "off the bottom right",
            point(5000, 5000),
            WidgetSize::Small,
            Some(slot(4, 2)),
        ),
        (
            "a medium at the right edge",
            point(820, 16),
            WidgetSize::Medium,
            Some(slot(3, 0)),
        ),
    ];
    for (name, corner, size, want) in cases {
        assert_eq!(snap(corner, size, metrics()), want, "{name}");
    }
    let tiny = GridMetrics {
        grid: GridSize {
            columns: 1,
            rows: 1,
        },
        ..metrics()
    };
    assert_eq!(snap(point(0, 0), WidgetSize::Medium, tiny), None);
}

#[test]
fn the_others_make_room_and_only_those_covered_move() {
    let grid = metrics().grid;
    let rest = [
        at(WidgetKind::Calendar, 4, 0),
        at(WidgetKind::Battery, 4, 1),
        at(WidgetKind::UpNext, 0, 0),
    ];
    let cases = [
        (
            "onto a free cell: nothing else moves",
            WidgetKind::Calendar,
            slot(2, 2),
            Some(vec![
                at(WidgetKind::Calendar, 2, 2),
                at(WidgetKind::Battery, 4, 1),
                at(WidgetKind::UpNext, 0, 0),
            ]),
        ),
        (
            "onto the Battery: it steps to the nearest free cell",
            WidgetKind::Calendar,
            slot(4, 1),
            Some(vec![
                at(WidgetKind::Calendar, 4, 1),
                at(WidgetKind::Battery, 4, 0),
                at(WidgetKind::UpNext, 0, 0),
            ]),
        ),
        (
            "a medium over two cells pushes the one it covers straight down (a diagonal is further)",
            WidgetKind::UpNext,
            slot(3, 1),
            Some(vec![
                at(WidgetKind::Calendar, 4, 0),
                at(WidgetKind::Battery, 4, 2),
                at(WidgetKind::UpNext, 3, 1),
            ]),
        ),
        (
            "back onto its own place: the rest as it was",
            WidgetKind::Battery,
            slot(4, 1),
            Some(rest.to_vec()),
        ),
    ];
    for (name, moving, to, want) in cases {
        assert_eq!(make_room(&rest, moving, to, grid), want, "{name}");
    }
}

#[test]
fn no_room_is_none() {
    // A 2 x 1 grid holding two small widgets: a medium dropped across both has nowhere to put
    // them.
    let grid = GridSize {
        columns: 2,
        rows: 1,
    };
    let full = [
        at(WidgetKind::Calendar, 0, 0),
        at(WidgetKind::Battery, 1, 0),
        at(WidgetKind::NowPlaying, 0, 5),
    ];
    assert_eq!(
        make_room(&full, WidgetKind::NowPlaying, slot(0, 0), grid),
        None
    );
    // Swapping two small ones is room enough.
    assert_eq!(
        make_room(&full[..2], WidgetKind::Calendar, slot(1, 0), grid),
        Some(vec![
            at(WidgetKind::Calendar, 1, 0),
            at(WidgetKind::Battery, 0, 0)
        ])
    );
}
