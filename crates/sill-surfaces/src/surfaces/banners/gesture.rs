//! A press on a card, pure: a press that moves less than [`SLOP`] is a click; past it, a drag
//! the banner machine follows (`DragStart`, `DragMove`, `DragEnd` with the release velocity),
//! and the click the pointer's release then makes is swallowed. The velocity is measured from
//! the move before the last one to the release, so a drag that pauses before letting go is
//! slow (it springs back) and a flick is fast (it flies out), with no time window of its own.
//!
//! Only the horizontal distance counts (design/13 §13.5: "swipe right to dismiss").

use sill_services::{Cpx, Stamp};

use super::deck::EntryKey;
use super::machine::BannerIn;

/// How far a press moves before it is a drag, logical px (a click's jitter stays under it).
pub const SLOP: f64 = 4.0;

/// Where the pointer was, when.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub x: f64,
    pub at: Stamp,
}

/// The press in progress.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Press {
    #[default]
    Idle,
    /// Down on `key`, not moved past the slop yet.
    Down { key: EntryKey, from: Sample },
    /// Dragging `key`: where it started, and the last two moves.
    Dragging {
        key: EntryKey,
        from: Sample,
        prev: Sample,
        last: Sample,
    },
    /// Released after a drag: the click that follows is not a click.
    Dragged,
}

/// Whether a click is the person's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Counts,
    Swallowed,
}

/// A button went down on `key` at `at`.
pub fn down(key: EntryKey, at: Sample) -> Press {
    Press::Down { key, from: at }
}

/// The pointer moved to `at` (anywhere on the column): the machine's inputs for the card.
pub fn moved(press: Press, at: Sample) -> (Press, Vec<(EntryKey, BannerIn)>) {
    match press {
        Press::Down { key, from } if (at.x - from.x).abs() >= SLOP => (
            Press::Dragging {
                key,
                from,
                prev: from,
                last: at,
            },
            vec![
                (key, BannerIn::DragStart),
                (key, BannerIn::DragMove { dx: dx(from, at) }),
            ],
        ),
        Press::Dragging {
            key, from, last, ..
        } => (
            Press::Dragging {
                key,
                from,
                prev: last,
                last: at,
            },
            vec![(key, BannerIn::DragMove { dx: dx(from, at) })],
        ),
        other => (other, Vec::new()),
    }
}

/// The button came up at `at`.
pub fn up(press: Press, at: Sample) -> (Press, Vec<(EntryKey, BannerIn)>) {
    match press {
        Press::Dragging { key, prev, .. } => (
            Press::Dragged,
            vec![(
                key,
                BannerIn::DragEnd {
                    velocity_px_s: velocity(prev, at),
                },
            )],
        ),
        Press::Down { .. } | Press::Idle | Press::Dragged => (Press::Idle, Vec::new()),
    }
}

/// A click arrived (after [`up`]): whether it is the person's.
pub fn click(press: Press) -> (Press, Click) {
    match press {
        Press::Dragged | Press::Dragging { .. } => (Press::Idle, Click::Swallowed),
        Press::Idle | Press::Down { .. } => (Press::Idle, Click::Counts),
    }
}

/// The distance from `from` to `to`, in the machine's hundredths.
fn dx(from: Sample, to: Sample) -> Cpx {
    Cpx(((to.x - from.x) * 100.0).round() as i32)
}

/// px/s from `from` to `to` (0 when no time passed).
fn velocity(from: Sample, to: Sample) -> i32 {
    match to.at.since(from.at) {
        0 => 0,
        ms => ((to.x - from.x) * 1000.0 / ms as f64).round() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::{Click, Press, Sample, click, down, moved, up};
    use crate::surfaces::banners::deck::EntryKey;
    use crate::surfaces::banners::machine::BannerIn;
    use sill_services::{Cpx, Stamp};

    const K: EntryKey = EntryKey(7);

    fn at(x: f64, ms: u64) -> Sample {
        Sample { x, at: Stamp(ms) }
    }

    /// Down at x 100 at 0, then each move, then up; every input and the click's fate.
    fn gesture(moves: &[(f64, u64)], release: (f64, u64)) -> (Vec<BannerIn>, Click) {
        let mut press = down(K, at(100.0, 0));
        let mut inputs = Vec::new();
        for &(x, ms) in moves {
            let (next, out) = moved(press, at(x, ms));
            press = next;
            inputs.extend(out.into_iter().map(|(_, i)| i));
        }
        let (press, out) = up(press, at(release.0, release.1));
        inputs.extend(out.into_iter().map(|(_, i)| i));
        (inputs, click(press).1)
    }

    #[test]
    fn a_still_press_is_a_click_and_a_drag_is_not() {
        type Moves = Vec<(f64, u64)>;
        type Case = (&'static str, Moves, (f64, u64), Vec<BannerIn>, Click);
        let cases: Vec<Case> = vec![
            ("still", vec![], (100.0, 80), vec![], Click::Counts),
            (
                "jitter under the slop",
                vec![(102.0, 20), (97.0, 40)],
                (98.0, 60),
                vec![],
                Click::Counts,
            ),
            (
                "a slow drag right, paused before the release",
                vec![(110.0, 100), (150.0, 400)],
                (150.0, 900),
                vec![
                    BannerIn::DragStart,
                    BannerIn::DragMove { dx: Cpx::px(10) },
                    BannerIn::DragMove { dx: Cpx::px(50) },
                    BannerIn::DragEnd { velocity_px_s: 50 },
                ],
                Click::Swallowed,
            ),
            (
                "a flick",
                vec![(110.0, 10), (130.0, 30)],
                (140.0, 40),
                vec![
                    BannerIn::DragStart,
                    BannerIn::DragMove { dx: Cpx::px(10) },
                    BannerIn::DragMove { dx: Cpx::px(30) },
                    BannerIn::DragEnd {
                        velocity_px_s: 1000,
                    },
                ],
                Click::Swallowed,
            ),
            (
                "left, damped later by the machine",
                vec![(80.0, 50)],
                (80.0, 100),
                vec![
                    BannerIn::DragStart,
                    BannerIn::DragMove { dx: Cpx::px(-20) },
                    BannerIn::DragEnd {
                        velocity_px_s: -200,
                    },
                ],
                Click::Swallowed,
            ),
        ];
        for (name, moves, release, want, fate) in cases {
            assert_eq!(gesture(&moves, release), (want, fate), "{name}");
        }
    }

    #[test]
    fn a_move_without_a_press_does_nothing() {
        assert_eq!(moved(Press::Idle, at(300.0, 5)), (Press::Idle, Vec::new()));
        assert_eq!(up(Press::Idle, at(300.0, 5)), (Press::Idle, Vec::new()));
    }
}
