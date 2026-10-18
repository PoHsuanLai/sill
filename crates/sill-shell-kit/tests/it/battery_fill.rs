//! design/23-WIDGETS.md section 4.1 on a real Blitz document, with design/30 section 1.3: a battery
//! ring stands at its level on mount, its percentage showing the true value at once; a new level
//! moves the arc linearly over `--t-move` while the number changes instantly; a charging bolt is
//! there from the first frame; once settled nothing moves and nothing asks for a frame (the
//! idle-frame rule); under Reduced motion the ring is at its level at once.

use dioxus::prelude::*;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::prelude::*;
use ds::style::tokens::timing::DurationToken;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_shell_kit::battery::ring::percent_text;
use sill_shell_kit::prelude::*;
use std::time::{Duration, Instant};

const VIEW: Viewport = Viewport {
    width: 200,
    height: 200,
    scale_percent: 100,
};

static LEVEL: GlobalSignal<Fraction> = Signal::global(|| Fraction(930));
/// A ring and its figure at `LEVEL`.
fn stage(motion: Motion, power: BatteryPower) -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent),
            BatteryRing { state: BatteryState { level: LEVEL(), power, low_at: LowAt::default() }, label: "This computer" }
            span { id: "figure", Label { text: percent_text(LEVEL()) } }
        }
    }
}

#[allow(non_snake_case)]
fn Stage() -> Element {
    stage(Motion::Standard, BatteryPower::Battery)
}

#[allow(non_snake_case)]
fn ReducedStage() -> Element {
    stage(Motion::Reduced, BatteryPower::Battery)
}

#[allow(non_snake_case)]
fn ChargingStage() -> Element {
    stage(Motion::Standard, BatteryPower::Charging)
}

/// How much of the circle the arc draws, in thousandths: 0 with no arc, 1000 for the whole
/// circle, else its end's angle clockwise from twelve (the arc starts at twelve).
fn drawn(harness: &Harness) -> u16 {
    let Some(d) = harness.attr(".ds-battery .ds-progress-fill path", "d") else {
        return 0;
    };
    if d.ends_with('Z') {
        return 1000;
    }
    let numbers: Vec<f32> = d
        .rsplit(' ')
        .take(2)
        .filter_map(|n| n.parse().ok())
        .collect();
    let [y, x] = numbers[..] else {
        panic!("an arc path without an end: {d}");
    };
    let degrees = (x - 50.0).atan2(50.0 - y).to_degrees().rem_euclid(360.0);
    (degrees / 360.0 * 1000.0).round() as u16
}

fn figure(harness: &Harness) -> String {
    harness.text_of("#figure").unwrap_or_default()
}

/// Step 10 ms at a time until the arc has been `target` for three frames running, recording every
/// arc seen on the way and when the last step landed.
fn samples(harness: &mut Harness, target: u16) -> (Vec<u16>, Instant) {
    let mut arcs: Vec<u16> = Vec::new();
    while arcs.len() < 3
        || arcs[arcs.len() - 3..]
            .iter()
            .any(|&arc| arc.abs_diff(target) > 1)
    {
        assert!(arcs.len() < 200, "the arc never reached {target}: {arcs:?}");
        arcs.push(drawn(harness));
        harness.advance(Duration::from_millis(10));
    }
    (arcs, harness.now())
}

fn travel() -> Duration {
    DurationToken::Move.duration(MotionLevel::Standard)
}

/// Settled for good: the same arc after a while, and nothing woke the document (no Rust timer
/// asked for a frame).
fn assert_rests(harness: &mut Harness) {
    let before = (
        harness.attr(".ds-battery .ds-progress-fill path", "d"),
        harness.wakes(),
    );
    harness.advance(Duration::from_millis(300));
    assert_eq!(
        (
            harness.attr(".ds-battery .ds-progress-fill path", "d"),
            harness.wakes()
        ),
        before,
        "a settled ring asks for frames"
    );
}

#[test]
fn a_ring_stands_at_its_level_and_shows_its_figure_at_once() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    assert_eq!(
        harness
            .attr(".ds-battery .ds-progress", "aria-valuenow")
            .as_deref(),
        Some("93")
    );
    assert!(
        (929..=931).contains(&drawn(&harness)),
        "{}",
        drawn(&harness)
    );
    assert_eq!(figure(&harness), "93%");
    assert_rests(&mut harness);
}

#[test]
fn a_new_level_moves_the_arc_linearly_and_the_figure_changes_at_once() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    let changed = harness.now();
    harness.within(|| *LEVEL.write() = Fraction(400));
    harness.advance(Duration::from_millis(1));
    assert_eq!(figure(&harness), "40%", "a number changes instantly");
    assert_eq!(
        harness
            .attr(".ds-battery .ds-progress", "aria-valuenow")
            .as_deref(),
        Some("40")
    );
    let (arcs, rested) = samples(&mut harness, 400);
    assert!(
        arcs.windows(2).all(|pair| pair[0] >= pair[1]),
        "the arc went back up: {arcs:?}"
    );
    assert!(
        arcs.iter().any(|&a| a > 410 && a < 920),
        "no frame between the old level and the new: {arcs:?}"
    );
    // The arc reaches the new level `travel()` after the 1 ms step above, and `samples` calls it
    // there once it has stood three 10 ms steps running: so a longer or shorter move fails.
    assert_eq!(
        rested.duration_since(changed),
        Duration::from_millis(1) + travel() + 3 * Duration::from_millis(10),
        "at rest only after the whole move"
    );
    assert!((399..=401).contains(&drawn(&harness)));
    assert_rests(&mut harness);
}

#[test]
fn a_charging_bolt_is_there_from_the_first_frame() {
    let mut harness = Harness::new(
        ChargingStage,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    assert_eq!(harness.count(".ds-battery-bolt"), 1);
    assert_eq!(harness.attr(".ds-battery-bolt", "style"), None, "fully in");
    assert_rests(&mut harness);
    // A change while charging keeps the bolt.
    harness.within(|| *LEVEL.write() = Fraction(950));
    // A charging ring leaves a gap at twelve for the bolt, so a full one ends at 914.
    samples(&mut harness, 914);
    assert_eq!(harness.count(".ds-battery-bolt"), 1);
}

#[test]
fn under_reduced_motion_the_ring_is_at_its_level_at_once() {
    let mut harness = Harness::new(
        ReducedStage,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    assert!((929..=931).contains(&drawn(&harness)), "{}", harness.html());
    assert_eq!(figure(&harness), "93%");
    harness.within(|| *LEVEL.write() = Fraction(400));
    harness.advance(Duration::from_millis(30));
    assert!((399..=401).contains(&drawn(&harness)), "{}", harness.html());
    assert_eq!(figure(&harness), "40%");
    assert!(!harness.is_animating());
}
