//! The auto-hide phase machine the dock and the bar share (design/10 §10.4 "Reveal"): pure, `now`
//! an argument, the consequence returned as a [`Change`]. A surface maps its own events to
//! [`Input`], applies the [`Change`] to its layer (input region, blur, frames, timer) and draws
//! [`slide_offset`]. What is not shared is the surface's own: the dock's busy states, the bar's
//! open popups (both are a [`Hold`]), and the regions each sets.
//!
//! The compositor's edge reveal in full screen ([`Input::Edge`]) is the compositor's own motion:
//! the machine goes straight between `Shown` and `Hidden` and the surface slides nowhere.

use ds::base::time::stamp::{Ms, Stamp};
use ds::prelude::Fraction;
use ds::style::tokens::easing::CubicBezier;
use sill_model::desktop::Shown as EdgeShown;
use sill_model::units::{Cpx, Permille};

/// The auto-hide phase. `Fixed` is auto-hide off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// Auto-hide is off: always shown, exclusive zone reserved.
    #[default]
    Fixed,
    /// Out of the way; only the trigger strip takes the pointer.
    Hidden,
    /// The pointer is in the trigger strip; shows after the delay.
    Armed {
        /// When the pointer arrived.
        since: Stamp,
    },
    /// Sliding in.
    Showing {
        /// When the slide began.
        since: Stamp,
    },
    /// Fully in.
    Shown,
    /// Sliding out.
    Hiding {
        /// When the slide began.
        since: Stamp,
    },
}

/// Whether the surface wants to stay shown whatever the pointer does (the dock's open menu or a
/// drag, the bar's open popup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Nothing keeps it: the pointer leaving hides it.
    Free,
    /// Something keeps it shown.
    Held,
}

/// One input to [`advance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// The pointer is on the surface (entered or moved), where the surface takes it.
    Inside,
    /// The pointer is off the surface, or whatever held it shown ended with the pointer off.
    Outside,
    /// The one-shot timer of [`Change::Arm`] fired.
    Tick,
    /// A frame callback.
    Frame,
    /// The compositor reveals or hides this edge's surface in full screen.
    Edge(EdgeShown),
}

/// What the surface must do about a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Wake with [`Input::Tick`] at this instant.
    Arm(Stamp),
    /// The slide in begins: take the shown regions and ask for frames.
    Show,
    /// The slide out begins: ask for frames.
    Hide,
    /// The slide out ended: take the strip's regions.
    Rest,
    /// Shown at once, no slide (the compositor moves it): take the shown regions.
    Pop,
    /// Hidden at once, no slide: take the strip's regions.
    Tuck,
}

/// The numbers the machine runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// How long the pointer rests before the slide in (`*.autohide_delay_ms`).
    pub delay: Ms,
    /// How long a slide takes (`*.autohide_slide_ms`); zero is instant.
    pub slide_ms: u64,
    /// The slide in's curve (`--e-out`).
    pub ease_out: CubicBezier,
    /// The slide out's curve (`--e-exit`).
    pub ease_exit: CubicBezier,
}

/// `curve` at time `t` (thousandths, at most all): quire's one evaluator, [`CubicBezier::at`].
pub fn eased(curve: CubicBezier, t: Permille) -> Permille {
    let at = Fraction(u16::try_from(t.0.min(1000)).unwrap_or(1000));
    Permille(u32::from(curve.at(at).0))
}

/// `phase` after `input` at `now`, and what the surface must do about it.
pub fn advance(
    phase: Phase,
    input: Input,
    now: Stamp,
    hold: Hold,
    t: &Timing,
) -> (Phase, Option<Change>) {
    match (phase, input) {
        (Phase::Fixed, _) => (Phase::Fixed, None),
        (Phase::Hidden, Input::Inside) => (
            Phase::Armed { since: now },
            Some(Change::Arm(now.after(t.delay))),
        ),
        (Phase::Armed { .. }, Input::Outside) => (Phase::Hidden, None),
        (Phase::Armed { since }, Input::Tick | Input::Frame) if now.since(since) >= t.delay.0 => {
            (Phase::Showing { since: now }, Some(Change::Show))
        }
        (Phase::Showing { since }, Input::Frame) if now.since(since) >= t.slide_ms => {
            (Phase::Shown, None)
        }
        (Phase::Shown, Input::Outside) if hold == Hold::Free => {
            (Phase::Hiding { since: now }, Some(Change::Hide))
        }
        (Phase::Hiding { since }, Input::Inside) => {
            // Reverse from the current offset: the show starts as far along as the hide was not.
            let done = now.since(since).min(t.slide_ms);
            (
                Phase::Showing {
                    since: Stamp(now.0.saturating_sub(t.slide_ms - done)),
                },
                None,
            )
        }
        (Phase::Hiding { since }, Input::Frame) if now.since(since) >= t.slide_ms => {
            (Phase::Hidden, Some(Change::Rest))
        }
        (
            Phase::Hidden | Phase::Armed { .. } | Phase::Showing { .. } | Phase::Hiding { .. },
            Input::Edge(EdgeShown::Shown),
        ) => (Phase::Shown, Some(Change::Pop)),
        (Phase::Armed { .. }, Input::Edge(EdgeShown::Hidden)) => (Phase::Hidden, None),
        (
            Phase::Showing { .. } | Phase::Shown | Phase::Hiding { .. },
            Input::Edge(EdgeShown::Hidden),
        ) if hold == Hold::Free => (Phase::Hidden, Some(Change::Tuck)),
        (other, _) => (other, None),
    }
}

/// Whether the slide needs a frame every refresh.
pub fn animating(phase: &Phase) -> bool {
    matches!(phase, Phase::Showing { .. } | Phase::Hiding { .. })
}

/// How far out of its place, towards `distance`, the surface is drawn at `now`.
pub fn slide_offset(phase: &Phase, now: Stamp, t: &Timing, distance: Cpx) -> Cpx {
    let along = |since: Stamp| {
        Permille(
            u32::try_from(now.since(since) * 1000 / t.slide_ms.max(1))
                .unwrap_or(1000)
                .min(1000),
        )
    };
    let scaled = |fraction: Permille| {
        Cpx((f64::from(distance.0) * f64::from(fraction.0) / 1000.0).round() as i32)
    };
    match *phase {
        Phase::Fixed | Phase::Shown => Cpx(0),
        Phase::Hidden | Phase::Armed { .. } => distance,
        Phase::Showing { since } => scaled(Permille(
            1000u32.saturating_sub(eased(t.ease_out, along(since)).0.min(1000)),
        )),
        Phase::Hiding { since } => scaled(Permille(eased(t.ease_exit, along(since)).0.min(1000))),
    }
}
