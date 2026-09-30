use sill_settings::WidgetKind;

use super::{DragInput, DragWorld, Hold, Landing, shown, step};
use crate::desktop::arrange::LayerPoint;
use crate::desktop::geometry::{GridMetrics, PlaceBox, place_box};
use crate::desktop::placement::GridSize;
use sill_ui_kit::widgets::grid::{GridSlot, Placement};
use sill_ui_kit::widgets::{WidgetHost, size_of};

fn metrics() -> GridMetrics {
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

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

fn point(x: i32, y: i32) -> LayerPoint {
    LayerPoint { x, y }
}

/// Calendar at the top right (820, 16), Battery under it (820, 196).
fn rest() -> Vec<Placement> {
    vec![
        at(WidgetKind::Calendar, 4, 0),
        at(WidgetKind::Battery, 4, 1),
    ]
}

fn boxes(rest: &[Placement]) -> Vec<(WidgetKind, PlaceBox)> {
    rest.iter()
        .map(|p| {
            let size = size_of(p.kind, WidgetHost::Desktop);
            (p.kind, place_box(p.slot, size, metrics()))
        })
        .collect()
}

/// Feed `inputs` from `Idle`: the last hold and every landing.
fn run(inputs: &[DragInput]) -> (Hold, Vec<Landing>) {
    let rest = rest();
    let boxes = boxes(&rest);
    let world = DragWorld {
        rest: &rest,
        boxes: &boxes,
        metrics: metrics(),
        threshold: 4,
    };
    inputs
        .iter()
        .fold((Hold::Idle, Vec::new()), |(hold, mut landed), input| {
            let (next, landing) = step(hold, *input, world);
            landed.push(landing);
            (next, landed)
        })
}

fn dropped(kind: WidgetKind, layout: Vec<Placement>) -> Landing {
    Landing::Dropped { kind, layout }
}

#[test]
fn only_travel_past_the_threshold_makes_a_press_a_drag() {
    use DragInput::{Down, Move, Up};
    let press = point(900, 100);
    let cases: [(&str, Vec<DragInput>, Vec<Landing>); 4] = [
        (
            "a click",
            vec![Down(press), Up(press)],
            vec![Landing::Nothing, Landing::Nothing],
        ),
        (
            "a wobble under the threshold is still a click",
            vec![Down(press), Move(point(902, 101)), Up(point(902, 101))],
            vec![Landing::Nothing; 3],
        ),
        (
            "a press between widgets is nothing",
            vec![
                Down(point(10, 10)),
                Move(point(300, 300)),
                Up(point(300, 300)),
            ],
            vec![Landing::Nothing; 3],
        ),
        (
            "past the threshold and back is still a drag, dropped where it began",
            vec![Down(press), Move(point(910, 100)), Up(press)],
            vec![
                Landing::Nothing,
                Landing::Nothing,
                dropped(WidgetKind::Calendar, rest()),
            ],
        ),
    ];
    for (name, inputs, want) in cases {
        let (hold, landed) = run(&inputs);
        assert_eq!(hold, Hold::Idle, "{name}");
        assert_eq!(landed, want, "{name}");
    }
}

#[test]
fn a_live_drag_follows_the_pointer_and_knows_its_cell() {
    use DragInput::{Down, Move};
    // Grabbed 80 px in from the Calendar's corner, dragged 540 px left and 360 down.
    let (hold, _) = run(&[Down(point(900, 96)), Move(point(360, 456))]);
    assert_eq!(
        hold,
        Hold::Live {
            kind: WidgetKind::Calendar,
            at: point(360, 456),
            grab: point(80, 80),
            over: Some(GridSlot { column: 1, row: 2 }),
        }
    );
    assert_eq!(hold.lifted(), Some((WidgetKind::Calendar, point(280, 376))));
}

#[test]
fn a_drop_snaps_and_the_covered_widget_makes_room() {
    use DragInput::{Down, Move, Up};
    let cases = [
        (
            "into a free cell",
            point(360, 456),
            vec![
                at(WidgetKind::Calendar, 1, 2),
                at(WidgetKind::Battery, 4, 1),
            ],
        ),
        (
            "onto the Battery, which moves up",
            point(905, 290),
            vec![
                at(WidgetKind::Calendar, 4, 1),
                at(WidgetKind::Battery, 4, 0),
            ],
        ),
        (
            "far off the layer: clamped to the nearest corner cell",
            point(-2000, 3000),
            vec![
                at(WidgetKind::Calendar, 0, 2),
                at(WidgetKind::Battery, 4, 1),
            ],
        ),
    ];
    for (name, to, want) in cases {
        let (_, landed) = run(&[Down(point(900, 96)), Move(point(890, 96)), Move(to), Up(to)]);
        assert_eq!(
            landed.last(),
            Some(&dropped(WidgetKind::Calendar, want)),
            "{name}"
        );
    }
}

#[test]
fn the_others_make_room_while_it_is_over_them_and_go_back_after() {
    let rest = rest();
    let over = |column, row| Hold::Live {
        kind: WidgetKind::Calendar,
        at: point(0, 0),
        grab: point(0, 0),
        over: Some(GridSlot { column, row }),
    };
    assert_eq!(
        shown(&over(4, 1), &rest, metrics()),
        vec![
            at(WidgetKind::Calendar, 4, 1),
            at(WidgetKind::Battery, 4, 0)
        ]
    );
    assert_eq!(
        shown(&over(2, 2), &rest, metrics()),
        vec![
            at(WidgetKind::Calendar, 2, 2),
            at(WidgetKind::Battery, 4, 1)
        ],
        "moved on: the Battery is back at its own place"
    );
    assert_eq!(shown(&Hold::Idle, &rest, metrics()), rest);
}
