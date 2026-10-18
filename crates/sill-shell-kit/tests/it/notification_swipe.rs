//! Swipe to dismiss on a real Blitz document: a drag released under the threshold
//! springs the card back and dismisses nothing; one released past it flies the card out and
//! reports `on_dismiss` at `settle(PanelOut)`, not before; a horizontal scroll is summed into
//! the same offset and decided once the deltas stop; the click that ends a drag never opens the
//! card; and under Reduced the flight settles at Reduced's length.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::notifications::parts::AppMark;
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

const VIEW: Viewport = Viewport {
    width: 480,
    height: 240,
    scale_percent: 100,
};

static LOG: GlobalSignal<Vec<String>> = Signal::global(Vec::new);
thread_local! {
    /// The motion setting the next harness on this thread is built with.
    static MOTION: Cell<Motion> = const { Cell::new(Motion::Standard) };
}

#[allow(non_snake_case)]
fn Card() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: MOTION.with(Cell::get), ..Appearance::default() }, material: Material::Toast,
            div { style: "padding:24px",
                NotificationCard {
                    app: AppMark { icon: IconSource::Glyph(Icon::Mail), name: "Mail".into() },
                    age: "now",
                    summary: "Ada Lovelace",
                    body: "The notes run longer than the memoir.",
                    on_close: |_| LOG.write().push("close".to_owned()),
                    on_open: |_| LOG.write().push("open".to_owned()),
                    swipe: NotificationSwipe::Dismiss(EventHandler::new(|()| LOG.write().push("dismiss".to_owned()))),
                }
            }
            p { class: "log", {LOG().join(",")} }
        }
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn start(motion: Motion) -> (Harness, Point) {
    MOTION.with(|cell| cell.set(motion));
    // On the virtual clock: the release velocity and the wheel's quiet window both read
    // ds::time, and on the wall clock they raced under load.
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| LOG.write().clear());
    harness.advance(ms(1));
    let at = harness
        .centre(".ds-notification-plate")
        .expect("the card is drawn");
    (harness, at)
}

fn log(harness: &mut Harness) -> Vec<String> {
    harness.within(|| LOG.peek().clone())
}

fn swipe(harness: &Harness) -> Option<String> {
    harness.attr(".ds-notification", "data-swipe")
}

fn style(harness: &Harness) -> String {
    harness
        .attr(".ds-notification", "style")
        .unwrap_or_default()
}

fn right(at: Point, dx: f32) -> Point {
    Point {
        x: Px(at.x.0 + dx),
        y: at.y,
    }
}

/// Whether `on_dismiss` has been heard, read off the page.
fn dismissed(harness: &Harness) -> bool {
    harness
        .text_of(".log")
        .is_some_and(|log| log.contains("dismiss"))
}

/// Drag from `at` to `dx` in four slow steps (100 ms apart: 10 px steps are 100 px/s, far under
/// the fling speed), then release there; the instant just before the release.
fn drag(harness: &mut Harness, at: Point, dx: f32) -> Instant {
    harness.send(Input::pointer_down(at));
    for step in 1..=4u8 {
        harness.advance(ms(100));
        harness.send(Input::pointer_move(right(at, dx * f32::from(step) / 4.0)));
    }
    assert_eq!(swipe(harness).as_deref(), Some("live"));
    assert!(
        style(harness).contains(&format!("--swipe-dx:{dx}px")),
        "the card follows the pointer 1:1: {}",
        style(harness)
    );
    let released = harness.now();
    harness.send(Input::pointer_up(right(at, dx)));
    released
}

#[test]
fn a_drag_released_under_the_threshold_springs_back() {
    let (mut harness, at) = start(Motion::Standard);
    let _ = drag(&mut harness, at, 40.0);
    assert_eq!(swipe(&harness).as_deref(), Some("rest"));
    // Driven motion (design/05 section 14): it springs home from where it was let go.
    assert!(
        style(&harness).contains("--swipe-dx:40px"),
        "it starts home from the release: {}",
        style(&harness)
    );
    settle_until(&mut harness, |h| !style(h).contains("--swipe-dx"));
    ds_harness::harness::assert_settles_to_zero_frames(&mut harness);
    harness.advance(ms(400));
    let entries = log(&mut harness);
    assert!(
        entries.is_empty(),
        "neither dismissed nor opened by the drag's click: {entries:?}"
    );
}

#[test]
fn a_drag_released_past_the_threshold_flies_out_and_reports_at_settle() {
    let (mut harness, at) = start(Motion::Standard);
    let flight = settle(Anim::PanelOut, MotionLevel::Standard);
    let released = drag(&mut harness, at, 100.0);
    assert_eq!(swipe(&harness).as_deref(), Some("gone"));
    assert!(
        style(&harness).contains("--swipe-dx:100px"),
        "it leaves from where it was"
    );
    harness.advance(flight / 2);
    assert!(
        log(&mut harness).is_empty(),
        "not before the flight has run"
    );
    let heard = settle_until(&mut harness, dismissed);
    assert_eq!(
        heard.duration_since(released),
        flight,
        "on_dismiss once the flight had run"
    );
    assert!(!log(&mut harness).contains(&"open".to_owned()));
}

#[test]
fn a_horizontal_scroll_is_summed_and_decided_when_it_stops() {
    let (mut harness, at) = start(Motion::Standard);
    // Short: 30 + 20 px, then quiet. Back to its place.
    harness.send(Input::wheel(at, Px(30.0), Px(0.0)));
    harness.send(Input::wheel(at, Px(20.0), Px(1.0)));
    assert!(
        style(&harness).contains("--swipe-dx:50px"),
        "{}",
        style(&harness)
    );
    assert_eq!(swipe(&harness).as_deref(), Some("live"));
    settle_until(&mut harness, |h| swipe(h).as_deref() == Some("rest"));
    assert!(log(&mut harness).is_empty());

    // Far: three deltas of 30 px summed to 90, then quiet: dismissed.
    for _ in 0..3 {
        harness.send(Input::wheel(at, Px(30.0), Px(0.0)));
        harness.advance(ms(16));
    }
    assert!(
        style(&harness).contains("--swipe-dx:90px"),
        "{}",
        style(&harness)
    );
    settle_until(&mut harness, |h| swipe(h).as_deref() == Some("gone"));
    settle_until(&mut harness, dismissed);
}

#[test]
fn under_reduced_the_flight_settles_at_reduceds_length() {
    let (mut harness, at) = start(Motion::Reduced);
    let reduced = settle(Anim::PanelOut, MotionLevel::Reduced);
    let standard = settle(Anim::PanelOut, MotionLevel::Standard);
    let released = drag(&mut harness, at, 100.0);
    let heard = settle_until(&mut harness, dismissed);
    let took = heard.duration_since(released);
    // Exactly Reduced's length: the flight is a quick exit, which Reduced does not stretch.
    assert_eq!(took, reduced);
    assert!(reduced <= standard, "{reduced:?} against {standard:?}");
}
