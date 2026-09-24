//! The hover label (design/10 §10.3.4, §10.4 "Label"): a first delay (`--d-fly`), then warm —
//! the next tile's label shows at once until the pointer has been out for `warm_ms`.

use sill_services::Stamp;

use super::params::DockParams;
use super::press::Press;
use super::types::{DockEffect, DockEvent, DockState, Hit, MenuState, TileId};

/// The label's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Label {
    #[default]
    Idle,
    /// The pointer settled on `tile` at `since`; the label shows after the fly delay.
    Pending {
        tile: TileId,
        since: Stamp,
    },
    Shown {
        tile: TileId,
    },
    /// A label showed recently: the next hover shows at once until `until`.
    Warm {
        until: Stamp,
    },
}

/// The label half of [`super::step`].
pub(crate) fn step(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    let quiet = dock.press != Press::Up || dock.menu == MenuState::Open;
    let label = match *event {
        DockEvent::Press { .. } => hide(dock.label, out),
        _ if quiet => hide(dock.label, out),
        DockEvent::PointerEnter { hit, .. } | DockEvent::PointerMove { hit, .. } => {
            hovered(dock.label, hit, now, p, out)
        }
        DockEvent::PointerLeave => left(dock.label, now, p, out),
        DockEvent::Frame | DockEvent::Tick => timed(dock.label, now, p, out),
        _ => dock.label,
    };
    DockState { label, ..dock }
}

fn hide(label: Label, out: &mut Vec<DockEffect>) -> Label {
    if matches!(label, Label::Shown { .. }) {
        out.push(DockEffect::HideLabel);
    }
    Label::Idle
}

fn hovered(label: Label, hit: Hit, now: Stamp, p: &DockParams, out: &mut Vec<DockEffect>) -> Label {
    let Some(tile) = hit.tile() else {
        return match label {
            Label::Pending { .. } => Label::Idle,
            other => other,
        };
    };
    let pending = |out: &mut Vec<DockEffect>| {
        out.push(DockEffect::WakeAt(now.after(p.fly_delay_ms)));
        Label::Pending { tile, since: now }
    };
    match label {
        Label::Idle => pending(out),
        Label::Pending { tile: t, .. } if t != tile => pending(out),
        Label::Shown { tile: t } if t != tile => {
            out.push(DockEffect::ShowLabel { tile });
            Label::Shown { tile }
        }
        Label::Warm { until } if now < until => {
            out.push(DockEffect::ShowLabel { tile });
            Label::Shown { tile }
        }
        Label::Warm { .. } => pending(out),
        same => same,
    }
}

fn left(label: Label, now: Stamp, p: &DockParams, out: &mut Vec<DockEffect>) -> Label {
    match label {
        Label::Shown { .. } => {
            let until = now.after(p.warm_ms);
            out.push(DockEffect::HideLabel);
            out.push(DockEffect::WakeAt(until));
            Label::Warm { until }
        }
        Label::Pending { .. } => Label::Idle,
        other => other,
    }
}

fn timed(label: Label, now: Stamp, p: &DockParams, out: &mut Vec<DockEffect>) -> Label {
    match label {
        Label::Pending { tile, since } if now.since(since) >= p.fly_delay_ms => {
            out.push(DockEffect::ShowLabel { tile });
            Label::Shown { tile }
        }
        Label::Warm { until } if now >= until => Label::Idle,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::Label;
    use crate::surfaces::dock::machine::params::tests::params;
    use crate::surfaces::dock::machine::types::{
        Button, DockEffect, DockEvent, DockState, Hit, Point, TileApp, TileId,
    };
    use crate::surfaces::dock::machine::{DropZone, step};
    use sill_services::{Cpx, Stamp};

    /// One scripted input sequence: `(ms, input)` pairs.
    type Script = Vec<(u64, DockEvent)>;

    fn over(tile: u32) -> DockEvent {
        DockEvent::PointerMove {
            at: Point {
                x: Cpx::px(100 + 52 * i32::try_from(tile).unwrap_or(0)),
                y: Cpx::px(40),
            },
            hit: Hit::Tile {
                tile: TileId(tile),
                app: TileApp::Inactive,
            },
            zone: DropZone::Slot(tile),
        }
    }

    fn labels(out: &[DockEffect]) -> Vec<DockEffect> {
        out.iter()
            .filter(|e| {
                matches!(
                    e,
                    DockEffect::ShowLabel { .. } | DockEffect::HideLabel | DockEffect::WakeAt(_)
                )
            })
            .cloned()
            .collect()
    }

    fn run(events: &[(u64, DockEvent)]) -> (Label, Vec<DockEffect>) {
        let p = params();
        let (dock, out) = events
            .iter()
            .fold((DockState::default(), Vec::new()), |(dock, _), &(ms, e)| {
                step(dock, e, Stamp(ms), &p)
            });
        (dock.label, labels(&out))
    }

    #[test]
    fn label_follows_design_10_acceptance_7() {
        let show = |ms| vec![(0, over(1)), (ms, DockEvent::Tick)];
        let cases: Vec<(&str, Script, Label, Vec<DockEffect>)> = vec![
            (
                "hovering arms the fly delay",
                vec![(0, over(1))],
                Label::Pending {
                    tile: TileId(1),
                    since: Stamp(0),
                },
                vec![DockEffect::WakeAt(Stamp(350))],
            ),
            (
                "not shown at 349 ms",
                show(349),
                Label::Pending {
                    tile: TileId(1),
                    since: Stamp(0),
                },
                vec![],
            ),
            (
                "shown at 350 ms",
                show(350),
                Label::Shown { tile: TileId(1) },
                vec![DockEffect::ShowLabel { tile: TileId(1) }],
            ),
            (
                "a neighbour shows at once",
                [show(350), vec![(400, over(2))]].concat(),
                Label::Shown { tile: TileId(2) },
                vec![DockEffect::ShowLabel { tile: TileId(2) }],
            ),
            (
                "leaving warms",
                [show(350), vec![(400, DockEvent::PointerLeave)]].concat(),
                Label::Warm { until: Stamp(800) },
                vec![DockEffect::HideLabel, DockEffect::WakeAt(Stamp(800))],
            ),
            (
                "back within 400 ms: 0 ms delay",
                [
                    show(350),
                    vec![(400, DockEvent::PointerLeave), (700, over(3))],
                ]
                .concat(),
                Label::Shown { tile: TileId(3) },
                vec![DockEffect::ShowLabel { tile: TileId(3) }],
            ),
            (
                "back after 500 ms: the fly delay again",
                [
                    show(350),
                    vec![
                        (400, DockEvent::PointerLeave),
                        (800, DockEvent::Tick),
                        (900, over(3)),
                    ],
                ]
                .concat(),
                Label::Pending {
                    tile: TileId(3),
                    since: Stamp(900),
                },
                vec![DockEffect::WakeAt(Stamp(1250))],
            ),
            (
                "moving to another tile while pending restarts the delay",
                vec![(0, over(1)), (200, over(2))],
                Label::Pending {
                    tile: TileId(2),
                    since: Stamp(200),
                },
                vec![DockEffect::WakeAt(Stamp(550))],
            ),
            (
                "a press hides it",
                [
                    show(350),
                    vec![(
                        400,
                        DockEvent::Press {
                            button: Button::Primary,
                            at: Point::default(),
                            hit: Hit::Tile {
                                tile: TileId(1),
                                app: TileApp::Inactive,
                            },
                        },
                    )],
                ]
                .concat(),
                Label::Idle,
                // The press arms hold-to-menu first; the label hides after.
                vec![DockEffect::WakeAt(Stamp(1000)), DockEffect::HideLabel],
            ),
        ];
        for (name, events, want, want_out) in cases {
            assert_eq!(run(&events), (want, want_out), "{name}");
        }
    }
}
