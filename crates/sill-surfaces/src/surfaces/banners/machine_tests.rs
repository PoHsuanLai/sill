//! Table tests for `banner.rs`.

use super::{
    Banner, BannerIn, BannerOut, BannerParams, Deadline, Leave, Persistence, damped, step,
};
use sill_services::{Cpx, Stamp};
use sill_settings::NotificationsSettings;

fn p() -> BannerParams {
    BannerParams::new(&NotificationsSettings::default())
}

fn run(persistence: Persistence, inputs: &[(u64, BannerIn)]) -> (Banner, Vec<BannerOut>) {
    let params = p();
    inputs.iter().fold(
        (Banner::Entering { since: Stamp(0) }, Vec::new()),
        |(b, mut all), &(ms, input)| {
            let (b, out) = step(b, input, Stamp(ms), persistence, &params);
            all.extend(out);
            (b, all)
        },
    )
}

fn left(b: &Banner) -> Option<Leave> {
    match b {
        Banner::Leaving { why, .. } => Some(*why),
        _ => None,
    }
}

#[test]
fn design_13_acceptance_9_lifetime_and_hover() {
    use Persistence::{Expires, Persists};
    let entered = (100, BannerIn::Entered);
    // Leaves at hold (5200 ms after entering), not before.
    assert_eq!(
        left(&run(Expires, &[entered, (5299, BannerIn::Tick)]).0),
        None
    );
    assert_eq!(
        left(&run(Expires, &[entered, (5300, BannerIn::Tick)]).0),
        Some(Leave::Timeout)
    );
    // Hovered at 4000 for 3 s: still present; leaves 1500 ms after the pointer leaves.
    let hovered = [
        entered,
        (4000, BannerIn::HoverIn),
        (7000, BannerIn::Tick),
        (7000, BannerIn::HoverOut),
    ];
    let (b, out) = run(Expires, &hovered);
    assert_eq!(
        b,
        Banner::Shown {
            deadline: Deadline::At(Stamp(8500))
        }
    );
    assert!(out.contains(&BannerOut::RequestTick(Stamp(8500))));
    assert_eq!(
        left(&run(Expires, &[&hovered[..], &[(8499, BannerIn::Tick)]].concat()).0),
        None
    );
    assert_eq!(
        left(&run(Expires, &[&hovered[..], &[(8500, BannerIn::Tick)]].concat()).0),
        Some(Leave::Timeout)
    );
    // An alert is still present after 60 s.
    assert_eq!(
        run(Persists, &[entered, (60_000, BannerIn::Tick)]).0,
        Banner::Shown {
            deadline: Deadline::Never
        }
    );
}

#[test]
fn a_long_remainder_is_kept_after_hover() {
    // Hovered at 1000 with 4300 ms left: resumes with 4300, not 1500.
    let (b, _) = run(
        Persistence::Expires,
        &[
            (100, BannerIn::Entered),
            (1000, BannerIn::HoverIn),
            (1200, BannerIn::HoverOut),
        ],
    );
    assert_eq!(
        b,
        Banner::Shown {
            deadline: Deadline::At(Stamp(5500))
        }
    );
}

#[test]
fn design_13_acceptance_10_swipe_dismiss() {
    let drag = |dx: i32, v: i32| {
        run(
            Persistence::Expires,
            &[
                (100, BannerIn::Entered),
                (500, BannerIn::DragStart),
                (600, BannerIn::DragMove { dx: Cpx::px(dx) }),
                (700, BannerIn::DragEnd { velocity_px_s: v }),
            ],
        )
    };
    assert_eq!(
        left(&drag(90, 100).0),
        Some(Leave::Dismissed),
        "90 px right"
    );
    assert_eq!(
        left(&drag(80, 0).0),
        Some(Leave::Dismissed),
        "exactly 80 px"
    );
    let (b, out) = drag(50, 100);
    assert!(
        matches!(b, Banner::Shown { .. }),
        "50 px slowly springs back"
    );
    assert!(out.contains(&BannerOut::SpringBack));
    assert_eq!(
        left(&drag(20, 650).0),
        Some(Leave::Dismissed),
        "a fast flick"
    );
    let swipe = run(
        Persistence::Expires,
        &[
            (100, BannerIn::Entered),
            (
                500,
                BannerIn::Swipe {
                    dx: Cpx::px(100),
                    velocity_px_s: 0,
                },
            ),
        ],
    );
    assert_eq!(
        left(&swipe.0),
        Some(Leave::Dismissed),
        "a 100 px horizontal scroll"
    );
}

#[test]
fn leftward_drags_are_damped() {
    let p = p();
    assert_eq!(damped(Cpx::px(40), &p), Cpx::px(40));
    assert_eq!(damped(Cpx::px(-40), &p), Cpx::px(-10));
}

#[test]
fn acting_or_closing_leaves_and_the_exit_removes() {
    for (input, why) in [
        (BannerIn::Clicked, Leave::Acted),
        (BannerIn::Action, Leave::Acted),
        (BannerIn::Close, Leave::Dismissed),
    ] {
        let (b, out) = run(
            Persistence::Expires,
            &[(100, BannerIn::Entered), (300, input)],
        );
        assert_eq!(left(&b), Some(why), "{input:?}");
        assert_eq!(out.last(), Some(&BannerOut::Leave(why)));
    }
    let (b, out) = run(
        Persistence::Expires,
        &[
            (100, BannerIn::Entered),
            (300, BannerIn::Close),
            (500, BannerIn::Left),
        ],
    );
    assert_eq!(b, Banner::Gone);
    assert_eq!(out.last(), Some(&BannerOut::Remove));
}
