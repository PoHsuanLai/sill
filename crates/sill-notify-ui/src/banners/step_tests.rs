//! Table tests for `step.rs`.

use super::{Banner, BannerIn, BannerOut, BannerParams, Deadline, Leave, step};
use ds::base::time::stamp::Stamp;
use sill_model::notifications::BannerLife;

fn p() -> BannerParams {
    BannerParams::default()
}

fn run(life: BannerLife, inputs: &[(u64, BannerIn)]) -> (Banner, Vec<BannerOut>) {
    let params = p();
    inputs.iter().fold(
        (Banner::Entering { since: Stamp(0) }, Vec::new()),
        |(b, mut all), &(ms, input)| {
            let (b, out) = step(b, input, Stamp(ms), life, &params);
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
    use BannerLife::{Expires, Persists};
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
        BannerLife::Expires,
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
fn a_swipe_leaves_from_any_live_phase() {
    let cases: Vec<(&str, Vec<(u64, BannerIn)>)> = vec![
        ("entering", vec![]),
        ("shown", vec![(100, BannerIn::Entered)]),
        (
            "hovered",
            vec![(100, BannerIn::Entered), (400, BannerIn::HoverIn)],
        ),
    ];
    for (name, before) in cases {
        let inputs = [&before[..], &[(500, BannerIn::Swiped)]].concat();
        let (b, out) = run(BannerLife::Expires, &inputs);
        assert_eq!(left(&b), Some(Leave::Swiped), "{name}");
        assert_eq!(out.last(), Some(&BannerOut::Leave(Leave::Swiped)), "{name}");
    }
}

#[test]
fn acting_or_closing_leaves_and_the_exit_removes() {
    for (input, why) in [
        (BannerIn::Clicked, Leave::Acted),
        (BannerIn::Action, Leave::Acted),
        (BannerIn::Close, Leave::Dismissed),
    ] {
        let (b, out) = run(
            BannerLife::Expires,
            &[(100, BannerIn::Entered), (300, input)],
        );
        assert_eq!(left(&b), Some(why), "{input:?}");
        assert_eq!(out.last(), Some(&BannerOut::Leave(why)));
    }
    let (b, out) = run(
        BannerLife::Expires,
        &[
            (100, BannerIn::Entered),
            (300, BannerIn::Close),
            (500, BannerIn::Left),
        ],
    );
    assert_eq!(b, Banner::Gone);
    assert_eq!(out.last(), Some(&BannerOut::Remove));
}
