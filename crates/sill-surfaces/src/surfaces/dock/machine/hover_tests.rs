//! Table tests for `hover.rs`.

use super::{FreezeCause, Hover, Moved, displayed, freeze, linear};
use crate::surfaces::dock::machine::params::tests::params;
use crate::surfaces::dock::machine::types::{DockEffect, DockEvent, DockState, Hit, Point};
use crate::surfaces::dock::machine::{DropZone, step};
use sill_services::{Cpx, Permille, Stamp};

/// One scripted input sequence: `(ms, input)` pairs.
type Script = Vec<(u64, DockEvent)>;

fn at(x: i32) -> Point {
    Point {
        x: Cpx::px(x),
        y: Cpx::px(40),
    }
}

fn mv(x: i32) -> DockEvent {
    DockEvent::PointerMove {
        at: at(x),
        hit: Hit::Pill,
        zone: DropZone::Slot(0),
    }
}

/// Feed `(ms, event)` pairs; the state and the effects of the last event.
fn run(events: &[(u64, DockEvent)]) -> (DockState, Vec<DockEffect>) {
    let p = params();
    events.iter().fold(
        (DockState::default(), Vec::new()),
        |(dock, _), &(ms, event)| step(dock, event, Stamp(ms), &p),
    )
}

#[test]
fn hover_transitions_follow_the_design_table() {
    let enter = (
        0,
        DockEvent::PointerEnter {
            at: at(100),
            hit: Hit::Pill,
        },
    );
    let cases: Vec<(&str, Script, Hover, Vec<DockEffect>)> = vec![
        (
            "enter starts entering from zero",
            vec![enter],
            Hover::Entering {
                from: Permille(0),
                since: Stamp(0),
            },
            vec![DockEffect::RequestFrames],
        ),
        (
            "a first move enters too (cosmic-comp #2230)",
            vec![(5, mv(100))],
            Hover::Entering {
                from: Permille(0),
                since: Stamp(5),
            },
            vec![DockEffect::RequestFrames],
        ),
        (
            "entering is still animating at 119 ms",
            vec![enter, (119, DockEvent::Frame)],
            Hover::Entering {
                from: Permille(0),
                since: Stamp(0),
            },
            vec![DockEffect::Redraw],
        ),
        (
            "entering completes at 120 ms",
            vec![enter, (120, DockEvent::Frame)],
            Hover::Tracking {
                moved: Moved::Drawn,
            },
            vec![DockEffect::Redraw],
        ),
        (
            "tracking with no move stops frames",
            vec![enter, (120, DockEvent::Frame), (136, DockEvent::Frame)],
            Hover::Tracking {
                moved: Moved::Drawn,
            },
            vec![DockEffect::StopFrames],
        ),
        (
            "a move while tracking asks for a frame",
            vec![enter, (120, DockEvent::Frame), (130, mv(140))],
            Hover::Tracking {
                moved: Moved::Pending,
            },
            vec![DockEffect::RequestFrames],
        ),
        (
            "a move to the same x is nothing",
            vec![enter, (120, DockEvent::Frame), (130, mv(100))],
            Hover::Tracking {
                moved: Moved::Drawn,
            },
            vec![],
        ),
        (
            "the pending move is drawn on the next frame",
            vec![
                enter,
                (120, DockEvent::Frame),
                (130, mv(140)),
                (136, DockEvent::Frame),
            ],
            Hover::Tracking {
                moved: Moved::Drawn,
            },
            vec![DockEffect::Redraw],
        ),
        (
            "leave while tracking leaves from full",
            vec![
                enter,
                (120, DockEvent::Frame),
                (300, DockEvent::PointerLeave),
            ],
            Hover::Leaving {
                from: Permille(1000),
                since: Stamp(300),
            },
            vec![DockEffect::RequestFrames],
        ),
        (
            "leave while entering leaves from where it got",
            vec![enter, (60, DockEvent::PointerLeave)],
            Hover::Leaving {
                from: Permille(500),
                since: Stamp(60),
            },
            vec![DockEffect::RequestFrames],
        ),
        (
            "leaving reaches rest after the leave time",
            vec![
                enter,
                (120, DockEvent::Frame),
                (300, DockEvent::PointerLeave),
                (500, DockEvent::Frame),
            ],
            Hover::Rest,
            vec![DockEffect::StopFrames],
        ),
        (
            "re-enter while leaving rises from the current value",
            vec![
                enter,
                (120, DockEvent::Frame),
                (300, DockEvent::PointerLeave),
                (350, mv(90)),
            ],
            Hover::Entering {
                from: Permille(750),
                since: Stamp(350),
            },
            vec![DockEffect::RequestFrames],
        ),
    ];
    for (name, events, want_hover, want_out) in cases {
        let (dock, out) = run(&events);
        assert_eq!(dock.hover, want_hover, "{name}");
        assert_eq!(out, want_out, "{name}");
    }
}

#[test]
fn re_entering_takes_the_remaining_enter_time() {
    // design/10 §10.3.3: remaining enter time = (1 - progress) x 120 ms.
    let p = params();
    let hover = Hover::Entering {
        from: Permille(750),
        since: Stamp(0),
    };
    assert_eq!(linear(&hover, Stamp(29), &p), Permille(991));
    assert_eq!(linear(&hover, Stamp(30), &p), Permille(1000));
}

#[test]
fn leaving_always_takes_the_leave_time() {
    let p = params();
    for from in [1000, 600, 100] {
        let hover = Hover::Leaving {
            from: Permille(from),
            since: Stamp(0),
        };
        assert!(linear(&hover, Stamp(199), &p).0 > 0, "from {from} at 199");
        assert_eq!(linear(&hover, Stamp(200), &p), Permille(0), "from {from}");
    }
}

#[test]
fn displayed_progress_is_eased_and_zero_when_magnification_is_off() {
    let p = params();
    let dock = DockState {
        hover: Hover::Entering {
            from: Permille(0),
            since: Stamp(0),
        },
        ..DockState::default()
    };
    // ease-out at 0.5 of the enter time is well past half.
    let (half, _) = displayed(&dock, Stamp(60), &p);
    assert!(half.0 > 850 && half.0 < 1000, "{half:?}");
    let off = crate::surfaces::dock::machine::DockParams {
        magnification: sill_settings::Magnification::Off,
        ..params()
    };
    assert_eq!(displayed(&dock, Stamp(60), &off).0, Permille(0));
}

#[test]
fn a_freeze_holds_the_frame_and_a_thaw_resumes_by_pointer() {
    let p = params();
    let dock = DockState {
        hover: Hover::Tracking {
            moved: Moved::Drawn,
        },
        ..DockState::default()
    };
    let frozen = freeze(dock, FreezeCause::Menu, Stamp(10), &p);
    assert!(matches!(
        frozen.hover,
        Hover::Frozen {
            shown: Permille(1000),
            linear: Permille(1000),
            cause: FreezeCause::Menu
        }
    ));
    // Moves do not change a frozen frame.
    let (still, _) = step(frozen.clone(), mv(300), Stamp(20), &p);
    assert_eq!(still.hover, frozen.hover);
    assert_eq!(still.x, frozen.x);
}
