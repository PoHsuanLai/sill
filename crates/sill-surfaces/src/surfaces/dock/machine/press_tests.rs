//! Table tests for `press.rs`.

use super::{ActivateMode, DropZone, Instance, Press};
use crate::surfaces::dock::machine::params::tests::params;
use crate::surfaces::dock::machine::types::{
    Button, DockEffect, DockEvent, DockKey, DockState, Hit, MenuState, Modifier, Point, TileApp,
    TileId,
};
use crate::surfaces::dock::machine::{DockParams, step};
use sill_services::{Cpx, Stamp};
use sill_settings::{ActiveClick, DockModifierClicks};

/// One scripted input sequence: `(ms, input)` pairs.
type Script = Vec<(u64, DockEvent)>;

const T: TileId = TileId(3);

fn pt(x: i32, y: i32) -> Point {
    Point {
        x: Cpx::px(x),
        y: Cpx::px(y),
    }
}

fn tile(app: TileApp) -> Hit {
    Hit::Tile { tile: T, app }
}

fn press(button: Button, app: TileApp) -> DockEvent {
    DockEvent::Press {
        button,
        at: pt(100, 40),
        hit: tile(app),
    }
}

fn release(button: Button, app: TileApp, modifier: Modifier) -> DockEvent {
    DockEvent::Release {
        button,
        at: pt(101, 40),
        hit: tile(app),
        modifier,
    }
}

fn click(button: Button, app: TileApp, modifier: Modifier) -> Vec<(u64, DockEvent)> {
    vec![
        (0, press(button, app)),
        (80, release(button, app, modifier)),
    ]
}

/// The effects of the last event that are about pressing (not frames, timers or labels).
fn acts(p: &DockParams, events: &[(u64, DockEvent)]) -> (DockState, Vec<DockEffect>) {
    let (dock, out) = events
        .iter()
        .fold((DockState::default(), Vec::new()), |(d, _), &(ms, e)| {
            step(d, e, Stamp(ms), p)
        });
    let keep = out
        .into_iter()
        .filter(|e| {
            !matches!(
                e,
                DockEffect::Redraw
                    | DockEffect::RequestFrames
                    | DockEffect::StopFrames
                    | DockEffect::WakeAt(_)
                    | DockEffect::HideLabel
            )
        })
        .collect();
    (dock, keep)
}

#[test]
fn clicks_do_what_design_10_3_6_says() {
    use Button::{Middle, Primary};
    use Modifier::{Alt, Cmd, CmdAlt, None};
    let activate = |mode| vec![DockEffect::Activate { tile: T, mode }];
    let cases: Vec<(&str, Script, Vec<DockEffect>)> = vec![
        (
            "not running launches",
            click(Primary, TileApp::NotRunning, None),
            vec![DockEffect::Launch {
                tile: T,
                instance: Instance::Existing,
            }],
        ),
        (
            "no windows reopens",
            click(Primary, TileApp::NoWindows, None),
            activate(ActivateMode::Reopen),
        ),
        (
            "all minimized unminimizes",
            click(Primary, TileApp::AllMinimized, None),
            activate(ActivateMode::Unminimize),
        ),
        (
            "inactive raises",
            click(Primary, TileApp::Inactive, None),
            activate(ActivateMode::Raise),
        ),
        (
            "active with two windows cycles",
            click(Primary, TileApp::Active { windows: 2 }, None),
            activate(ActivateMode::Cycle),
        ),
        (
            "active with one window does nothing",
            click(Primary, TileApp::Active { windows: 1 }, None),
            vec![],
        ),
        (
            "cmd-click shows in files",
            click(Primary, TileApp::Inactive, Cmd),
            vec![DockEffect::ShowInFiles { tile: T }],
        ),
        (
            "option-click hides the current app",
            click(Primary, TileApp::Inactive, Alt),
            activate(ActivateMode::HideCurrent),
        ),
        (
            "option-cmd-click hides the others",
            click(Primary, TileApp::Inactive, CmdAlt),
            activate(ActivateMode::HideOthers),
        ),
        (
            "option-click on a stopped app launches",
            click(Primary, TileApp::NotRunning, Alt),
            vec![DockEffect::Launch {
                tile: T,
                instance: Instance::Existing,
            }],
        ),
        (
            "middle click launches a new instance",
            click(Middle, TileApp::Inactive, None),
            vec![DockEffect::Launch {
                tile: T,
                instance: Instance::New,
            }],
        ),
        (
            "release on another tile cancels",
            vec![
                (0, press(Primary, TileApp::Inactive)),
                (
                    80,
                    DockEvent::Release {
                        button: Primary,
                        at: pt(160, 40),
                        hit: Hit::Tile {
                            tile: TileId(4),
                            app: TileApp::Inactive,
                        },
                        modifier: None,
                    },
                ),
            ],
            vec![],
        ),
        (
            "release outside cancels",
            vec![
                (0, press(Primary, TileApp::Inactive)),
                (
                    80,
                    DockEvent::Release {
                        button: Primary,
                        at: pt(100, -40),
                        hit: Hit::Outside,
                        modifier: None,
                    },
                ),
            ],
            vec![],
        ),
    ];
    let p = params();
    for (name, events, want) in cases {
        let (dock, got) = acts(&p, &events);
        assert_eq!(got, want, "{name}");
        assert_eq!(dock.press, Press::Up, "{name}");
    }
}

#[test]
fn settings_change_what_a_click_does() {
    let nothing = DockParams {
        click_active_app: ActiveClick::Nothing,
        ..params()
    };
    assert_eq!(
        acts(
            &nothing,
            &click(
                Button::Primary,
                TileApp::Active { windows: 3 },
                Modifier::None
            )
        )
        .1,
        vec![]
    );
    let plain = DockParams {
        modifier_clicks: DockModifierClicks::Off,
        ..params()
    };
    assert_eq!(
        acts(
            &plain,
            &click(Button::Primary, TileApp::Inactive, Modifier::Cmd)
        )
        .1,
        vec![DockEffect::Activate {
            tile: T,
            mode: ActivateMode::Raise
        }]
    );
}

#[test]
fn a_launch_click_starts_a_launch_bounce() {
    let (dock, _) = acts(
        &params(),
        &click(Button::Primary, TileApp::NotRunning, Modifier::None),
    );
    assert_eq!(dock.bounces.len(), 1);
    assert_eq!(dock.bounces[0].0, T);
}

#[test]
fn menus_open_on_secondary_press_and_on_a_held_press() {
    let p = params();
    let cases: Vec<(&str, Script, Vec<DockEffect>)> = vec![
        (
            "secondary press",
            vec![(0, press(Button::Secondary, TileApp::Inactive))],
            vec![DockEffect::OpenMenu { tile: T }],
        ),
        (
            "held 599 ms: nothing yet",
            vec![
                (0, press(Button::Primary, TileApp::Inactive)),
                (599, DockEvent::Tick),
            ],
            vec![],
        ),
        (
            "held 600 ms",
            vec![
                (0, press(Button::Primary, TileApp::Inactive)),
                (600, DockEvent::Tick),
            ],
            vec![DockEffect::OpenMenu { tile: T }],
        ),
    ];
    for (name, events, want) in cases {
        assert_eq!(acts(&p, &events).1, want, "{name}");
    }
    let (dock, _) = acts(&p, &[(0, press(Button::Secondary, TileApp::Inactive))]);
    assert_eq!(dock.menu, MenuState::Open);
    let (closed, _) = step(dock, DockEvent::MenuClosed, Stamp(10), &p);
    assert_eq!(closed.menu, MenuState::Closed);
}

#[test]
fn drags_reorder_remove_or_cancel() {
    let p = params();
    let start = |zone| {
        vec![
            (0, press(Button::Primary, TileApp::Inactive)),
            (
                20,
                DockEvent::PointerMove {
                    at: pt(105, 44),
                    hit: tile(TileApp::Inactive),
                    zone,
                },
            ),
        ]
    };
    let cases: Vec<(&str, Script, Vec<DockEffect>)> = vec![
        (
            "release inside reorders",
            [
                start(DropZone::Slot(1)),
                vec![(
                    90,
                    release(Button::Primary, TileApp::Inactive, Modifier::None),
                )],
            ]
            .concat(),
            vec![DockEffect::Reorder { tile: T, to: 1 }],
        ),
        (
            "release beyond the threshold unpins",
            [
                start(DropZone::Remove),
                vec![(
                    90,
                    release(Button::Primary, TileApp::Inactive, Modifier::None),
                )],
            ]
            .concat(),
            vec![DockEffect::Unpin { tile: T }],
        ),
        (
            "escape cancels",
            [
                start(DropZone::Slot(0)),
                vec![(90, DockEvent::Key(DockKey::Escape))],
            ]
            .concat(),
            vec![],
        ),
    ];
    for (name, events, want) in cases {
        let (dock, got) = acts(&p, &events);
        assert_eq!(got, want, "{name}");
        assert_eq!(dock.press, Press::Up, "{name}");
    }
    // Eight px Manhattan is still a click; nine is a drag.
    let (still, _) = acts(
        &p,
        &[
            (0, press(Button::Primary, TileApp::Inactive)),
            (
                10,
                DockEvent::PointerMove {
                    at: pt(104, 44),
                    hit: tile(TileApp::Inactive),
                    zone: DropZone::Slot(3),
                },
            ),
        ],
    );
    assert!(matches!(still.press, Press::Down { .. }));
    let (drag, _) = acts(
        &p,
        &[
            (0, press(Button::Primary, TileApp::Inactive)),
            (
                10,
                DockEvent::PointerMove {
                    at: pt(105, 44),
                    hit: tile(TileApp::Inactive),
                    zone: DropZone::Slot(3),
                },
            ),
        ],
    );
    assert!(matches!(drag.press, Press::Dragging { .. }));
}

#[test]
fn spring_loading_opens_a_running_app_after_the_delay() {
    let p = params();
    let dnd = |app| DockEvent::DndMotion {
        x: Cpx::px(100),
        hit: tile(app),
    };
    let cases: Vec<(&str, Script, Vec<DockEffect>)> = vec![
        (
            "499 ms: not yet",
            vec![(0, dnd(TileApp::Inactive)), (499, DockEvent::Tick)],
            vec![],
        ),
        (
            "500 ms: opens",
            vec![(0, dnd(TileApp::Inactive)), (500, DockEvent::Tick)],
            vec![DockEffect::SpringOpen { tile: T }],
        ),
        (
            "left before the delay",
            vec![
                (0, dnd(TileApp::Inactive)),
                (400, DockEvent::DndLeave),
                (500, DockEvent::Tick),
            ],
            vec![],
        ),
        (
            "a stopped app never springs",
            vec![(0, dnd(TileApp::NotRunning)), (600, DockEvent::Tick)],
            vec![],
        ),
    ];
    for (name, events, want) in cases {
        assert_eq!(acts(&p, &events).1, want, "{name}");
    }
}
