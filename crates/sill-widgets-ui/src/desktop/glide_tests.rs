use std::time::{Duration, Instant};

use ds::motion::detail::touch::Touch;
use ds::motion::spring_point::Release;
use ds::motion::velocity::Velocity;
use ds::prelude::*;

use super::{Ask, Glide, PointMeter, Pose, Seen, ask, drawn, to_layer, to_point};
use crate::desktop::arrange::LayerPoint;

fn lp(x: i32, y: i32) -> LayerPoint {
    LayerPoint { x, y }
}

fn release(x: i32, y: i32) -> Release {
    Release {
        x: Velocity(x),
        y: Velocity(y),
    }
}

#[test]
fn each_change_of_hold_or_cell_asks_the_spring_once() {
    let cell = lp(560, 196);
    let other = lp(1100, 16);
    let hand = lp(600, 230);
    let thrown = Glide::Thrown {
        touch: Touch::Remote,
        release: release(-900, 40),
    };
    let seen = |target, pose| Seen { target, pose };
    let cases = [
        (
            "picked up: stand at the hand",
            seen(cell, Pose::Resting),
            seen(cell, Pose::Lifted(hand)),
            Glide::Aside,
            Ask::Hold(hand),
        ),
        (
            "held and moved: the render draws the hand",
            seen(cell, Pose::Lifted(hand)),
            seen(other, Pose::Lifted(lp(700, 300))),
            Glide::Aside,
            Ask::Nothing,
        ),
        (
            "let go: from the hand at its velocity",
            seen(other, Pose::Lifted(hand)),
            seen(cell, Pose::Resting),
            thrown,
            Ask::Drop {
                from: hand,
                to: cell,
                touch: Touch::Remote,
                release: release(-900, 40),
            },
        ),
        (
            "let go with no throw known: a still remote drop",
            seen(cell, Pose::Lifted(hand)),
            seen(cell, Pose::Resting),
            Glide::Aside,
            Ask::Drop {
                from: hand,
                to: cell,
                touch: Touch::Remote,
                release: Release::default(),
            },
        ),
        (
            "its cell moved: aside from where it is",
            seen(cell, Pose::Resting),
            seen(other, Pose::Resting),
            thrown,
            Ask::Aside(other),
        ),
        (
            "nothing changed",
            seen(cell, Pose::Resting),
            seen(cell, Pose::Resting),
            thrown,
            Ask::Nothing,
        ),
    ];
    for (name, before, now, glide, want) in cases {
        assert_eq!(ask(before, now, glide), want, "{name}");
    }
}

#[test]
fn the_hand_is_drawn_until_the_spring_has_it() {
    let hand = lp(600, 230);
    let spring = lp(1100, 16);
    let drop = Ask::Drop {
        from: hand,
        to: lp(560, 196),
        touch: Touch::Remote,
        release: Release::default(),
    };
    let cases = [
        (Pose::Lifted(hand), Ask::Hold(hand), hand),
        (Pose::Lifted(hand), Ask::Nothing, hand),
        (Pose::Resting, drop, hand),
        (Pose::Resting, Ask::Aside(lp(0, 0)), spring),
        (Pose::Resting, Ask::Nothing, spring),
    ];
    for (pose, asked, want) in cases {
        assert_eq!(drawn(pose, asked, spring), want, "{pose:?} {asked:?}");
    }
}

#[test]
fn points_round_to_whole_px() {
    let cases = [
        ((0.0, 0.0), lp(0, 0)),
        ((559.6, 195.4), lp(560, 195)),
        ((-3.5, 12.5), lp(-4, 13)),
    ];
    for ((x, y), want) in cases {
        let got = to_layer(Point { x: Px(x), y: Px(y) });
        assert_eq!(got, want, "{x},{y}");
        assert_eq!(to_layer(to_point(want)), want);
    }
}

#[test]
fn a_release_reads_both_axes_of_the_last_moves() {
    let t0 = Instant::now();
    let ms = |n: u64| t0 + Duration::from_millis(n);
    let meter = PointMeter::pressed(lp(1182, 98), ms(0))
        .moved(lp(1162, 98), ms(16))
        .moved(lp(1142, 106), ms(32));
    let cases = [
        (
            "released where it last moved",
            meter,
            lp(1142, 106),
            ms(40),
            release(-1250, 500),
        ),
        (
            "released somewhere new counts as a last move",
            meter,
            lp(1122, 106),
            ms(48),
            release(-1250, 0),
        ),
        (
            "the pointer had stopped",
            meter,
            lp(1142, 106),
            ms(200),
            Release::default(),
        ),
        (
            "a press let go at once",
            PointMeter::pressed(lp(10, 10), ms(0)),
            lp(10, 10),
            ms(5),
            Release::default(),
        ),
    ];
    for (name, meter, at, when, want) in cases {
        assert_eq!(meter.released(at, when), want, "{name}");
    }
}
