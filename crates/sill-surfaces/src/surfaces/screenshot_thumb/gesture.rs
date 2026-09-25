//! A drag out of the card: a press that moves left or up past the swipe's threshold hands the
//! file over ([`super::start_file_drag`]) once per press. Rightwards is quire's swipe, which
//! keeps the file and dismisses the thumbnail. Pure, table-tested.

use ds::{Point, Px};

/// Where a press on the card is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Gesture {
    /// No button down.
    #[default]
    Idle,
    /// Down at this point, not yet a drag out.
    Pressed(Point),
    /// This press has already asked for a drag out.
    Handed,
}

/// What the pointer did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureIn {
    Down(Point),
    /// A move with the primary button held.
    Move(Point),
    /// Released, left the card, or moved with no button down.
    Up,
}

/// Whether to start a drag out now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragOut {
    Nothing,
    Start,
}

/// One step: a move `threshold` or more to the left or up from the press asks for a drag out.
pub fn gesture_step(gesture: Gesture, input: GestureIn, threshold: Px) -> (Gesture, DragOut) {
    match (gesture, input) {
        (_, GestureIn::Down(at)) => (Gesture::Pressed(at), DragOut::Nothing),
        (_, GestureIn::Up) => (Gesture::Idle, DragOut::Nothing),
        (Gesture::Pressed(from), GestureIn::Move(to)) => {
            let left = from.x.0 - to.x.0;
            let up = from.y.0 - to.y.0;
            match left.max(up) >= threshold.0 {
                true => (Gesture::Handed, DragOut::Start),
                false => (gesture, DragOut::Nothing),
            }
        }
        (Gesture::Idle | Gesture::Handed, GestureIn::Move(_)) => (gesture, DragOut::Nothing),
    }
}

#[cfg(test)]
mod tests {
    use ds::{Point, Px};

    use super::{DragOut, Gesture, GestureIn, gesture_step};

    fn at(x: f32, y: f32) -> Point {
        Point { x: Px(x), y: Px(y) }
    }

    #[test]
    fn a_press_moved_left_or_up_past_the_threshold_drags_out_once() {
        let pressed = Gesture::Pressed(at(100.0, 100.0));
        let cases = [
            (
                Gesture::Idle,
                GestureIn::Down(at(1.0, 2.0)),
                Gesture::Pressed(at(1.0, 2.0)),
                DragOut::Nothing,
            ),
            (
                Gesture::Idle,
                GestureIn::Move(at(0.0, 0.0)),
                Gesture::Idle,
                DragOut::Nothing,
            ),
            (
                pressed,
                GestureIn::Move(at(40.0, 100.0)),
                pressed,
                DragOut::Nothing,
            ),
            (
                pressed,
                GestureIn::Move(at(20.0, 100.0)),
                Gesture::Handed,
                DragOut::Start,
            ),
            (
                pressed,
                GestureIn::Move(at(100.0, 10.0)),
                Gesture::Handed,
                DragOut::Start,
            ),
            (
                pressed,
                GestureIn::Move(at(300.0, 100.0)),
                pressed,
                DragOut::Nothing,
            ),
            (
                pressed,
                GestureIn::Move(at(100.0, 300.0)),
                pressed,
                DragOut::Nothing,
            ),
            (
                Gesture::Handed,
                GestureIn::Move(at(0.0, 0.0)),
                Gesture::Handed,
                DragOut::Nothing,
            ),
            (
                Gesture::Handed,
                GestureIn::Up,
                Gesture::Idle,
                DragOut::Nothing,
            ),
        ];
        for (from, input, want, out) in cases {
            assert_eq!(
                gesture_step(from, input, Px(80.0)),
                (want, out),
                "{from:?} + {input:?}"
            );
        }
    }
}
