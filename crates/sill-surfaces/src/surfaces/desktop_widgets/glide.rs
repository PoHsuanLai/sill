//! What a widget's spring does next (FINDINGS "Drag glide on springs"), decided from how it was
//! held last render and now, and the hand's release velocity. Pure: `placed` hands the answer to
//! quire's `SpringPointMotion`, which owns the motion (its springs, Reduced motion, the frames).

use std::time::Instant;

use ds::detail::Touch;
use ds::motion::{Release, VelocityMeter};
use ds::{Point, Px};

use super::arrange::LayerPoint;

/// Where the widget is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// At its cell (springing there when the cell changed).
    Resting,
    /// Picked up, its corner here.
    Lifted(LayerPoint),
}

/// How the widget was let go, for the spring into its cell after a drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glide {
    /// Not the widget the hand let go: it only moves aside for another.
    Aside,
    /// The widget the hand let go: `touch` is the release's contact, `release` its velocity.
    Thrown { touch: Touch, release: Release },
}

/// What one render saw: the widget's cell corner and how it was held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    pub target: LayerPoint,
    pub pose: Pose,
}

/// What the spring is asked to do this render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Nothing changed.
    Nothing,
    /// Picked up: stand at the hand's corner (stops a glide still running; no frames while
    /// held, the render draws the pointer's corner itself).
    Hold(LayerPoint),
    /// Let go at `from`: spring to `to` from there, at the release's velocity.
    Drop {
        from: LayerPoint,
        to: LayerPoint,
        touch: Touch,
        release: Release,
    },
    /// Its cell moved (making room, or back): spring there from where it is, at the speed it has.
    Aside(LayerPoint),
}

/// The ask from `before` to `now`; `glide` says how a drop was let go.
pub fn ask(before: Seen, now: Seen, glide: Glide) -> Ask {
    match (before.pose, now.pose) {
        (Pose::Resting, Pose::Lifted(held)) => Ask::Hold(held),
        (Pose::Lifted(_), Pose::Lifted(_)) => Ask::Nothing,
        (Pose::Lifted(from), Pose::Resting) => {
            let (touch, release) = match glide {
                Glide::Thrown { touch, release } => (touch, release),
                Glide::Aside => (Touch::Remote, Release::default()),
            };
            Ask::Drop {
                from,
                to: now.target,
                touch,
                release,
            }
        }
        (Pose::Resting, Pose::Resting) if before.target != now.target => Ask::Aside(now.target),
        (Pose::Resting, Pose::Resting) => Ask::Nothing,
    }
}

/// Where to draw this render, before the spring has taken the ask: the hand's corner while held
/// and on the release's render (the spring starts there the moment after), else the spring's.
pub fn drawn(now: Pose, asked: Ask, spring: LayerPoint) -> LayerPoint {
    match (now, asked) {
        (Pose::Lifted(held), _) => held,
        (Pose::Resting, Ask::Drop { from, .. }) => from,
        (Pose::Resting, Ask::Nothing | Ask::Hold(_) | Ask::Aside(_)) => spring,
    }
}

/// A layer point as quire's.
pub fn to_point(at: LayerPoint) -> Point {
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}

/// Quire's point in whole layer px.
pub fn to_layer(at: Point) -> LayerPoint {
    let px = |v: f32| v.round().clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    LayerPoint {
        x: px(at.x.0),
        y: px(at.y.0),
    }
}

/// A drag's pointer moves, both axes, for the release velocity (quire's `VelocityMeter` per
/// axis).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointMeter {
    x: VelocityMeter,
    y: VelocityMeter,
    last: Option<LayerPoint>,
}

impl PointMeter {
    /// A new drag, pressed at `at` at `when`.
    pub fn pressed(at: LayerPoint, when: Instant) -> PointMeter {
        PointMeter::default().moved(at, when)
    }

    /// The pointer is at `at` at `when`.
    pub fn moved(self, at: LayerPoint, when: Instant) -> PointMeter {
        let p = to_point(at);
        PointMeter {
            x: self.x.moved(p.x, when),
            y: self.y.moved(p.y, when),
            last: Some(at),
        }
    }

    /// The release velocity for a release at `at` at `when`. A release where the last move was
    /// (the usual case) reads the moves before it; one somewhere new counts as a last move.
    pub fn released(self, at: LayerPoint, when: Instant) -> Release {
        let meter = match self.last {
            Some(last) if last == at => self,
            Some(_) | None => self.moved(at, when),
        };
        Release {
            x: meter.x.released(when),
            y: meter.y.released(when),
        }
    }
}

#[cfg(test)]
#[path = "glide_tests.rs"]
mod tests;
