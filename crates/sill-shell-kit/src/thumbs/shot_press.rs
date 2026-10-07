//! A press on the screenshot thumbnail's picture, as a pure machine: a press that
//! travels [`DRAG_THRESHOLD`] becomes a drag, told to the caller once, and the click that ends
//! it does not open the picture; one that stays under the threshold opens on its click. The
//! host does the drag itself (a data source and a drag icon are the compositor's), so quire's
//! part ends at saying when it began and where. On a card that swipes away to the right
//! ([`DragLane::NotRight`]), a press that crosses the threshold heading mostly right is the
//! swipe's, not a drag: it starts none and does not open either.

use ds::base::geometry::units::Point;
use ds::motion::drag::{DRAG_THRESHOLD, Drag, DragPhase};

/// A drag out of the thumbnail began: the press point, and where the pointer crossed the
/// threshold. Client coordinates, as the pointer events report them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragStart {
    /// Where the press went down.
    pub from: Point,
    /// Where the pointer was when the press became a drag.
    pub at: Point,
}

/// What the current (or last) press has been so far: the click that ends it reads this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Gesture {
    /// Still a tap: its click opens.
    Tap,
    /// Became a drag: its click is swallowed.
    Dragged,
    /// Crossed the threshold into the swipe's lane: no drag, and its click is swallowed.
    Swiped,
}

/// Which way a press may become a drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DragLane {
    /// Any way.
    Any,
    /// Any way but mostly to the right, which is the swipe to dismiss.
    NotRight,
}

impl DragLane {
    /// Whether a crossing from `from` to `at` is a drag in this lane.
    fn admits(self, from: Point, at: Point) -> bool {
        let (dx, dy) = (at.x.0 - from.x.0, at.y.0 - from.y.0);
        match self {
            DragLane::Any => true,
            DragLane::NotRight => !(dx > 0.0 && dx.abs() >= dy.abs()),
        }
    }
}

/// The picture's press.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShotPress {
    drag: Drag<()>,
    gesture: Gesture,
}

impl Default for ShotPress {
    fn default() -> Self {
        ShotPress {
            drag: Drag::new(DRAG_THRESHOLD),
            gesture: Gesture::Tap,
        }
    }
}

/// What the pointer did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PressInput {
    /// Went down on the picture.
    Down(Point),
    /// Moved.
    Moved(Point, DragLane),
    /// Came up, or was cancelled.
    Up,
}

impl ShotPress {
    /// One step, and the drag start it causes, if this is the move that crossed the threshold.
    pub(crate) fn step(self, input: PressInput) -> (Self, Option<DragStart>) {
        match input {
            PressInput::Down(at) => (
                ShotPress {
                    drag: self.drag.down((), at),
                    gesture: Gesture::Tap,
                },
                None,
            ),
            PressInput::Moved(at, lane) => self.moved(at, lane),
            PressInput::Up => (
                ShotPress {
                    drag: self.drag.up().0,
                    ..self
                },
                None,
            ),
        }
    }

    fn moved(self, at: Point, lane: DragLane) -> (Self, Option<DragStart>) {
        let from = match self.drag.phase() {
            DragPhase::Pending { from, .. } => Some(*from),
            DragPhase::Idle | DragPhase::Live { .. } => None,
        };
        let drag = self.drag.moved(at);
        match (from, drag.phase()) {
            (Some(from), DragPhase::Live { .. }) if lane.admits(from, at) => (
                ShotPress {
                    drag,
                    gesture: Gesture::Dragged,
                },
                Some(DragStart { from, at }),
            ),
            (Some(_), DragPhase::Live { .. }) => (
                ShotPress {
                    drag,
                    gesture: Gesture::Swiped,
                },
                None,
            ),
            _ => (ShotPress { drag, ..self }, None),
        }
    }

    /// Whether the click ending this press opens the picture: only a press that never dragged.
    pub(crate) fn opens(&self) -> bool {
        self.gesture == Gesture::Tap
    }
}

#[cfg(test)]
mod tests {
    use super::{DragLane, DragStart, PressInput, ShotPress};
    use ds::base::geometry::units::{Point, Px};

    fn at(x: f32, y: f32) -> Point {
        Point { x: Px(x), y: Px(y) }
    }

    /// A step of a test: the machine's input with the lane left out.
    #[derive(Clone, Copy)]
    enum I {
        Down(Point),
        Moved(Point),
        Up,
    }

    /// Run `inputs` from rest: the drag starts they caused, and whether a click then opens.
    fn run(inputs: &[I]) -> (Vec<DragStart>, bool) {
        run_in(DragLane::Any, inputs)
    }

    fn run_in(lane: DragLane, inputs: &[I]) -> (Vec<DragStart>, bool) {
        let mut press = ShotPress::default();
        let mut started = Vec::new();
        for &input in inputs {
            let input = match input {
                I::Down(at) => PressInput::Down(at),
                I::Moved(at) => PressInput::Moved(at, lane),
                I::Up => PressInput::Up,
            };
            let (next, start) = press.step(input);
            press = next;
            started.extend(start);
        }
        (started, press.opens())
    }

    #[test]
    fn a_drag_starts_once_past_the_threshold_and_its_click_does_not_open() {
        let crossed = DragStart {
            from: at(10.0, 10.0),
            at: at(13.0, 10.0),
        };
        #[rustfmt::skip]
        let cases: [(&str, Vec<I>, Vec<DragStart>, bool); 6] = [
            ("a tap", vec![I::Down(at(10.0, 10.0)), I::Up], vec![], true),
            ("2 px is still a tap", vec![I::Down(at(10.0, 10.0)), I::Moved(at(11.0, 11.0)), I::Up], vec![], true),
            ("3 px drags", vec![I::Down(at(10.0, 10.0)), I::Moved(at(13.0, 10.0)), I::Up], vec![crossed], false),
            ("once", vec![I::Down(at(10.0, 10.0)), I::Moved(at(13.0, 10.0)), I::Moved(at(40.0, 40.0)), I::Up], vec![crossed], false),
            ("a move with no press", vec![I::Moved(at(40.0, 40.0))], vec![], true),
            ("the next press is a tap again", vec![I::Down(at(10.0, 10.0)), I::Moved(at(30.0, 10.0)), I::Up, I::Down(at(5.0, 5.0)), I::Up], vec![DragStart { from: at(10.0, 10.0), at: at(30.0, 10.0) }], true),
        ];
        for (name, inputs, want, opens) in cases {
            assert_eq!(run(&inputs), (want, opens), "{name}");
        }
    }

    #[test]
    fn beside_a_swipe_a_rightward_crossing_is_the_swipes() {
        let from = at(10.0, 10.0);
        #[rustfmt::skip]
        let cases = [
            ("right", at(20.0, 12.0), vec![], false),
            ("left", at(0.0, 12.0), vec![DragStart { from, at: at(0.0, 12.0) }], false),
            ("up", at(12.0, 0.0), vec![DragStart { from, at: at(12.0, 0.0) }], false),
            ("down and a little right", at(13.0, 20.0), vec![DragStart { from, at: at(13.0, 20.0) }], false),
        ];
        for (name, to, want, opens) in cases {
            let inputs = [I::Down(from), I::Moved(to), I::Up];
            assert_eq!(run_in(DragLane::NotRight, &inputs), (want, opens), "{name}");
        }
    }
}
