//! Auto-hide (design/10 §10.3.11, §10.4 "Reveal", §10.5.5). `Fixed` is auto-hide off.

use shell_host::{BlurSpec, ElementId, ExclusiveZone, RegionSpec};
use sill_services::{Cpx, Permille, Stamp, ease};

use super::params::DockParams;
use super::press::Press;
use super::types::{DockEffect, DockEvent, DockState, Hit, MenuState};

/// The auto-hide phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reveal {
    /// Auto-hide is off: always shown, exclusive zone reserved.
    #[default]
    Fixed,
    Hidden,
    /// The pointer is in the trigger strip; shows after the delay.
    Armed {
        since: Stamp,
    },
    Showing {
        since: Stamp,
    },
    Shown,
    Hiding {
        since: Stamp,
    },
}

/// Which machines an event reaches: a hidden dock only watches its strip.
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
    if hidden && pointer {
        Gate::Reveal
    } else {
        Gate::Dock
    }
}

/// Whether the slide needs a frame every refresh.
pub(crate) fn animating(reveal: &Reveal) -> bool {
    matches!(reveal, Reveal::Showing { .. } | Reveal::Hiding { .. })
}

/// How far below its rest position the dock is drawn at `now` (design/10 §10.5.5).
pub fn slide_offset(reveal: &Reveal, now: Stamp, p: &DockParams) -> Cpx {
    let distance = f64::from(p.slide_distance.0);
    let t = |since: Stamp| {
        Permille(
            u32::try_from(now.since(since) * 1000 / p.slide_ms.max(1))
                .unwrap_or(1000)
                .min(1000),
        )
    };
    let scaled =
        |fraction: Permille| Cpx((distance * f64::from(fraction.0) / 1000.0).round() as i32);
    match *reveal {
        Reveal::Fixed | Reveal::Shown => Cpx(0),
        Reveal::Hidden | Reveal::Armed { .. } => p.slide_distance,
        Reveal::Showing { since } => scaled(Permille(
            1000u32.saturating_sub(ease(p.ease_out, t(since)).0.min(1000)),
        )),
        Reveal::Hiding { since } => scaled(Permille(ease(p.ease_exit, t(since)).0.min(1000))),
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
    let inside = |hit: &Hit| *hit != Hit::Outside;
    let busy = dock.menu == MenuState::Open || matches!(dock.press, Press::Dragging { .. });
    let reveal = match (dock.reveal, *event) {
        (Reveal::Fixed, _) => Reveal::Fixed,
        (
            Reveal::Hidden,
            DockEvent::PointerEnter { hit, .. } | DockEvent::PointerMove { hit, .. },
        ) if inside(&hit) => {
            out.push(DockEffect::WakeAt(now.after(p.autohide_delay_ms)));
            Reveal::Armed { since: now }
        }
        (Reveal::Armed { .. }, DockEvent::PointerLeave) => Reveal::Hidden,
        (Reveal::Armed { since }, DockEvent::Tick | DockEvent::Frame)
            if now.since(since) >= p.autohide_delay_ms =>
        {
            out.push(DockEffect::SetInput(hit_region()));
            out.push(DockEffect::SetBlur(BlurSpec::Region(pill_region())));
            out.push(DockEffect::RequestFrames);
            Reveal::Showing { since: now }
        }
        (Reveal::Showing { since }, DockEvent::Frame) if now.since(since) >= p.slide_ms => {
            Reveal::Shown
        }
        (Reveal::Shown, DockEvent::PointerLeave) if !busy => {
            out.push(DockEffect::RequestFrames);
            Reveal::Hiding { since: now }
        }
        (Reveal::Shown, DockEvent::MenuClosed)
            if dock.pointer == super::types::Pointer::Outside =>
        {
            out.push(DockEffect::RequestFrames);
            Reveal::Hiding { since: now }
        }
        (
            Reveal::Hiding { since },
            DockEvent::PointerEnter { .. } | DockEvent::PointerMove { .. },
        ) => {
            // Reverse from the current offset: the show starts as far along as the hide was not.
            let done = now.since(since).min(p.slide_ms);
            Reveal::Showing {
                since: Stamp(now.0.saturating_sub(p.slide_ms - done)),
            }
        }
        (Reveal::Hiding { since }, DockEvent::Frame) if now.since(since) >= p.slide_ms => {
            out.push(DockEffect::SetInput(strip_region()));
            out.push(DockEffect::SetBlur(BlurSpec::None));
            Reveal::Hidden
        }
        (other, _) => other,
    };
    DockState { reveal, ..dock }
}

/// The input region while shown: the `dock-hit` element (design/10 §10.3.12).
pub(crate) fn hit_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new("dock-hit"),
        radius: 0,
        inflate: 0,
    }
}

/// The blur region while shown: the `dock-pill` element with the pill's radius.
pub(crate) fn pill_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new("dock-pill"),
        radius: 22,
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
    use crate::surfaces::dock::machine::params::tests::params;
    use crate::surfaces::dock::machine::types::{DockEvent, DockState, Hit, Point};
    use crate::surfaces::dock::machine::{DropZone, step};
    use sill_services::{Cpx, Stamp};

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
    fn the_offset_runs_from_the_full_distance_to_zero() {
        let p = params();
        assert_eq!(slide_offset(&Reveal::Hidden, Stamp(0), &p), Cpx::px(80));
        assert_eq!(slide_offset(&Reveal::Fixed, Stamp(0), &p), Cpx(0));
        let showing = Reveal::Showing { since: Stamp(0) };
        assert_eq!(slide_offset(&showing, Stamp(0), &p), Cpx::px(80));
        assert_eq!(slide_offset(&showing, Stamp(500), &p), Cpx(0));
        let hiding = Reveal::Hiding { since: Stamp(0) };
        assert_eq!(slide_offset(&hiding, Stamp(0), &p), Cpx(0));
        assert_eq!(slide_offset(&hiding, Stamp(500), &p), Cpx::px(80));
    }
}
