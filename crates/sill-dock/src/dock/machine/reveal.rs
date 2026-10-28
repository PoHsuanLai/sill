//! Auto-hide (design/10 §10.3.11, §10.4 "Reveal", §10.5.5). `Fixed` is auto-hide off.

use ds::base::time::stamp::Stamp;
use shell_host::{BlurSpec, ElementId, ExclusiveZone, RegionSpec};
use sill_model::units::Cpx;
use sill_ui_kit::autohide::{self, Change, Hold, Input, advance};

use super::params::DockParams;
use super::press::Press;
use super::types::{DockEffect, DockEvent, DockState, Hit};

/// The auto-hide phase: `Fixed` is auto-hide off (the shared machine, `sill_ui_kit::autohide`).
pub use sill_ui_kit::autohide::Phase as Reveal;

/// Which machines an event reaches: a hidden dock only watches its strip, and the compositor's
/// edge reveal is the reveal's alone.
pub(crate) enum Gate {
    Reveal,
    Dock,
}

pub(crate) fn gate(dock: &DockState, event: &DockEvent) -> Gate {
    let hidden = matches!(dock.reveal, Reveal::Hidden | Reveal::Armed { .. });
    let pointer = matches!(
        event,
        DockEvent::PointerEnter { .. } | DockEvent::PointerMove { .. } | DockEvent::PointerLeave
    );
    if (hidden && pointer) || matches!(event, DockEvent::Edge(_)) {
        Gate::Reveal
    } else {
        Gate::Dock
    }
}

/// Whether the slide needs a frame every refresh.
pub(crate) fn animating(reveal: &Reveal) -> bool {
    autohide::animating(reveal)
}

/// How far below its rest position the dock is drawn at `now` (design/10 §10.5.5).
pub fn slide_offset(reveal: &Reveal, now: Stamp, p: &DockParams) -> Cpx {
    autohide::slide_offset(reveal, now, &p.timing(), p.slide_distance)
}

/// The shared machine's input for `event`, when the reveal cares about it.
fn input_of(dock: &DockState, event: &DockEvent) -> Option<Input> {
    let hiding = matches!(dock.reveal, Reveal::Hiding { .. });
    match *event {
        DockEvent::PointerEnter { hit, .. } | DockEvent::PointerMove { hit, .. }
            if hiding || hit != Hit::Outside =>
        {
            Some(Input::Inside)
        }
        DockEvent::PointerLeave => Some(Input::Outside),
        DockEvent::MenuClosed if !dock.pointer_inside => Some(Input::Outside),
        DockEvent::Tick => Some(Input::Tick),
        DockEvent::Frame => Some(Input::Frame),
        DockEvent::Edge(shown) => Some(Input::Edge(shown)),
        _ => None,
    }
}

/// The reveal half of [`super::step`].
pub(crate) fn step(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    let hold = if dock.menu_open || matches!(dock.press, Press::Dragging { .. }) {
        Hold::Held
    } else {
        Hold::Free
    };
    let Some(input) = input_of(&dock, event) else {
        return dock;
    };
    let (reveal, change) = advance(dock.reveal, input, now, hold, &p.timing());
    if let Some(change) = change {
        apply(change, p, out);
    }
    DockState { reveal, ..dock }
}

/// What the dock's layer does for `change`.
fn apply(change: Change, p: &DockParams, out: &mut Vec<DockEffect>) {
    let shown = |out: &mut Vec<DockEffect>| {
        out.push(DockEffect::SetInput(hit_region()));
        out.push(DockEffect::SetBlur(BlurSpec::Region(pill_region(
            p.pill_blur_radius,
        ))));
    };
    match change {
        Change::Arm(at) => out.push(DockEffect::WakeAt(at)),
        Change::Show => {
            shown(out);
            out.push(DockEffect::RequestFrames);
        }
        Change::Pop => shown(out),
        Change::Hide => out.push(DockEffect::RequestFrames),
        Change::Rest | Change::Tuck => {
            out.push(DockEffect::SetInput(strip_region()));
            out.push(DockEffect::SetBlur(BlurSpec::None));
        }
    }
}

/// The input region while shown: the `dock-hit` element (design/10 §10.3.12).
pub(crate) fn hit_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new("dock-hit"),
        radius: 0,
        inflate: 0,
    }
}

/// The blur region while shown: the `dock-pill` element, its corner `radius` (the circle the
/// pill's squircle shadows follow, `geometry::pill_blur_radius`).
pub(crate) fn pill_region(radius: i32) -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new("dock-pill"),
        radius,
        inflate: 0,
    }
}

/// The input region while hidden: the trigger strip element.
pub(crate) fn strip_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new("dock-strip"),
        radius: 0,
        inflate: 0,
    }
}

/// Auto-hide on reserves nothing; off reserves `gut + H_p` (design/10 §10.3.1).
pub fn exclusive_zone(reveal: &Reveal, reserve_px: u32) -> ExclusiveZone {
    match reveal {
        Reveal::Fixed => ExclusiveZone::Reserve(reserve_px),
        _ => ExclusiveZone::Reserve(0),
    }
}

#[cfg(test)]
mod tests {
    use super::{Reveal, slide_offset};
    use crate::dock::machine::params::tests::params;
    use crate::dock::machine::types::{DockEvent, DockState, Hit, Point};
    use crate::dock::machine::{DropZone, step};
    use ds::base::time::stamp::Stamp;
    use sill_model::units::Cpx;

    /// One scripted input sequence: `(ms, input)` pairs.
    type Script = Vec<(u64, DockEvent)>;

    fn strip() -> DockEvent {
        DockEvent::PointerMove {
            at: Point {
                x: Cpx::px(500),
                y: Cpx::px(158),
            },
            hit: Hit::Pill,
            zone: DropZone::Slot(0),
        }
    }

    fn run(events: &[(u64, DockEvent)]) -> Reveal {
        let p = params();
        events
            .iter()
            .fold(DockState::new(Reveal::Hidden), |d, &(ms, e)| {
                step(d, e, Stamp(ms), &p).0
            })
            .reveal
    }

    #[test]
    fn auto_hide_follows_design_10_acceptance_13() {
        let cases: Vec<(&str, Script, Reveal)> = vec![
            (
                "the strip arms",
                vec![(0, strip())],
                Reveal::Armed { since: Stamp(0) },
            ),
            (
                "190 ms: still hidden",
                vec![(0, strip()), (190, DockEvent::Tick)],
                Reveal::Armed { since: Stamp(0) },
            ),
            (
                "200 ms: the slide starts",
                vec![(0, strip()), (200, DockEvent::Tick)],
                Reveal::Showing { since: Stamp(200) },
            ),
            (
                "leaving the strip disarms",
                vec![(0, strip()), (100, DockEvent::PointerLeave)],
                Reveal::Hidden,
            ),
            (
                "the slide completes in 500 ms",
                vec![
                    (0, strip()),
                    (200, DockEvent::Tick),
                    (700, DockEvent::Frame),
                ],
                Reveal::Shown,
            ),
            (
                "leaving starts the hide at once",
                vec![
                    (0, strip()),
                    (200, DockEvent::Tick),
                    (700, DockEvent::Frame),
                    (900, DockEvent::PointerLeave),
                ],
                Reveal::Hiding { since: Stamp(900) },
            ),
            (
                "the hide ends hidden",
                vec![
                    (0, strip()),
                    (200, DockEvent::Tick),
                    (700, DockEvent::Frame),
                    (900, DockEvent::PointerLeave),
                    (1400, DockEvent::Frame),
                ],
                Reveal::Hidden,
            ),
            (
                "coming back mid-hide reverses",
                vec![
                    (0, strip()),
                    (200, DockEvent::Tick),
                    (700, DockEvent::Frame),
                    (900, DockEvent::PointerLeave),
                    (1000, strip()),
                ],
                Reveal::Showing { since: Stamp(600) },
            ),
        ];
        for (name, events, want) in cases {
            assert_eq!(run(&events), want, "{name}");
        }
    }

    #[test]
    fn the_compositors_edge_reveal_shows_and_hides_with_no_slide() {
        use super::{hit_region, strip_region};
        use crate::dock::machine::DockEffect;
        use sill_model::desktop::Shown;
        let edge = |shown| DockEvent::Edge(shown);
        let cases: Vec<(&str, Script, Reveal)> = vec![
            (
                "a reveal shows the hidden dock at once",
                vec![(0, edge(Shown::Shown))],
                Reveal::Shown,
            ),
            (
                "a reveal while armed shows it too",
                vec![(0, strip()), (50, edge(Shown::Shown))],
                Reveal::Shown,
            ),
            (
                "a hide puts it back at once",
                vec![(0, edge(Shown::Shown)), (100, edge(Shown::Hidden))],
                Reveal::Hidden,
            ),
            (
                "a hide of a dock already hidden changes nothing",
                vec![(0, edge(Shown::Hidden))],
                Reveal::Hidden,
            ),
        ];
        for (name, events, want) in cases {
            assert_eq!(run(&events), want, "{name}");
        }
        let p = params();
        let (shown, effects) = step(
            DockState::new(Reveal::Hidden),
            edge(Shown::Shown),
            Stamp(0),
            &p,
        );
        assert!(effects.contains(&DockEffect::SetInput(hit_region())));
        let (_, effects) = step(shown, edge(Shown::Hidden), Stamp(1), &p);
        assert!(effects.contains(&DockEffect::SetInput(strip_region())));
        // A dock that never hides has nothing to reveal.
        let (fixed, effects) = step(
            DockState::new(Reveal::Fixed),
            edge(Shown::Shown),
            Stamp(0),
            &p,
        );
        assert_eq!(fixed.reveal, Reveal::Fixed);
        assert!(effects.is_empty());
    }

    #[test]
    fn the_offset_runs_from_the_full_distance_to_zero() {
        let p = params();
        assert_eq!(slide_offset(&Reveal::Hidden, Stamp(0), &p), Cpx::px(76));
        assert_eq!(slide_offset(&Reveal::Fixed, Stamp(0), &p), Cpx(0));
        let showing = Reveal::Showing { since: Stamp(0) };
        assert_eq!(slide_offset(&showing, Stamp(0), &p), Cpx::px(76));
        assert_eq!(slide_offset(&showing, Stamp(500), &p), Cpx(0));
        let hiding = Reveal::Hiding { since: Stamp(0) };
        assert_eq!(slide_offset(&hiding, Stamp(0), &p), Cpx(0));
        assert_eq!(slide_offset(&hiding, Stamp(500), &p), Cpx::px(76));
    }
}
