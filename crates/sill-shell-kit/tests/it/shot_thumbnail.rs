//! The screenshot thumbnail on a real Blitz document: shown, the card rises in
//! (`data-presence="entering"`, `rise` at `--t-big`) and comes to rest; hidden, it slides out and
//! `on_hidden` runs only once `settle(PanelOut)` has passed. A press on the picture that travels
//! the drag threshold calls `ondrag` once and does not open; one that stays under it opens on its
//! click. The pointer on the card is told to `onhover` and shows the actions. With swipe to
//! dismiss on, a drag to the right is the swipe's (it flies out and reports at
//! `settle(PanelOut)`, never a drag out), while a drag to the left still starts a drag out.

use dioxus::prelude::*;
use ds::components::content::image_source::ImageSize;
use ds::motion::drag::DRAG_THRESHOLD;
use ds::prelude::*;
use ds_harness::harness::{SETTLE_BOUND, settle_until};
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_shell_kit::notifications::parts::Hover;
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use sill_shell_kit::thumbs::shot_press::DragStart;
use sill_shell_kit::thumbs::shot_thumbnail::ThumbAction;
use std::time::{Duration, Instant};

static SHOWN: GlobalSignal<Shown> = Signal::global(|| Shown::Visible);
static HIDDEN: GlobalSignal<u32> = Signal::global(|| 0);
static OPENED: GlobalSignal<u32> = Signal::global(|| 0);
static DRAGS: GlobalSignal<Vec<DragStart>> = Signal::global(Vec::new);
static HOVERS: GlobalSignal<Vec<Hover>> = Signal::global(Vec::new);
static DELETED: GlobalSignal<u32> = Signal::global(|| 0);
static SWIPED: GlobalSignal<u32> = Signal::global(|| 0);
static SWIPE_SHOWN: GlobalSignal<Shown> = Signal::global(|| Shown::Visible);

const VIEW: Viewport = Viewport {
    width: 400,
    height: 300,
    scale_percent: 100,
};

/// A 16 x 9 grey PNG, made here: nothing from anybody's screen.
fn picture() -> ImageSource {
    let grey = image::RgbaImage::from_pixel(16, 9, image::Rgba([120, 130, 140, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    grey.write_to(&mut bytes, image::ImageFormat::Png)
        .expect("a PNG encodes");
    ImageSource::png(&bytes.into_inner())
}

#[allow(non_snake_case)]
fn Thumb() -> Element {
    let actions = vec![ThumbAction {
        icon: Icon::Trash,
        label: "Delete".into(),
        onpress: EventHandler::new(|()| *DELETED.write() += 1),
    }];
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            ShotThumbnail {
                image: picture(),
                size: ImageSize { width: 16, height: 9 },
                shown: SHOWN(),
                on_hidden: move |()| *HIDDEN.write() += 1,
                actions,
                onopen: move |()| *OPENED.write() += 1,
                ondrag: move |start| DRAGS.write().push(start),
                onhover: move |hover| HOVERS.write().push(hover),
            }
        }
    }
}

#[allow(non_snake_case)]
fn Swipeable() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "padding-left:80px",
                ShotThumbnail {
                    image: picture(),
                    size: ImageSize { width: 16, height: 9 },
                    shown: SWIPE_SHOWN(),
                    on_hidden: move |()| *HIDDEN.write() += 1,
                    onopen: move |()| *OPENED.write() += 1,
                    ondrag: move |start| DRAGS.write().push(start),
                    swipe: NotificationSwipe::Dismiss(EventHandler::new(|()| *SWIPED.write() += 1)),
                }
            }
        }
    }
}

fn presence(harness: &Harness) -> Option<String> {
    harness.attr(".ds-shot", "data-presence")
}

fn read<T: Clone>(harness: &mut Harness, signal: &'static GlobalSignal<T>) -> T {
    harness.within(|| signal.peek().clone())
}

fn rested() -> Harness {
    let mut harness = Harness::new(Thumb, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    settle_until(&mut harness, |h| presence(h).as_deref() == Some("present"));
    harness
}

fn picture_centre(harness: &Harness) -> Point {
    harness
        .centre(".ds-shot-picture")
        .expect("the picture is laid out")
}

fn offset(at: Point, dx: f32, dy: f32) -> Point {
    Point {
        x: Px(at.x.0 + dx),
        y: Px(at.y.0 + dy),
    }
}

#[test]
fn it_rises_in_and_on_hidden_runs_only_after_the_slide_out_settles() {
    let mut harness = Harness::new(Thumb, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    assert_eq!(presence(&harness).as_deref(), Some("entering"));
    settle_until(&mut harness, |h| presence(h).as_deref() == Some("present"));
    let card = harness.rect(".ds-shot").expect("the card is laid out");
    assert!((card.size.width.0 - 240.0).abs() < 0.5, "{card:?}");
    let before = read(&mut harness, &HIDDEN);
    let out = settle(Anim::PanelOut, MotionLevel::Standard);
    let hiding = harness.now();
    harness.within(|| *SHOWN.write() = Shown::Hidden);
    harness.advance(Duration::from_millis(1));
    assert_eq!(presence(&harness).as_deref(), Some("leaving"));
    harness.advance(out / 2);
    assert_eq!(
        read(&mut harness, &HIDDEN),
        before,
        "not at half of settle(PanelOut)"
    );
    let gone = settle_until(&mut harness, |h| presence(h).is_none());
    assert_eq!(gone.duration_since(hiding), out);
    assert_eq!(read(&mut harness, &HIDDEN), before + 1);
    assert_eq!(
        harness.attr(".ds-shot", "data-shown").as_deref(),
        Some("hidden")
    );
}

#[test]
fn a_press_drags_only_past_the_threshold_and_then_does_not_open() {
    let mut harness = rested();
    let from = picture_centre(&harness);
    harness.send(Input::pointer_move(from));
    harness.send(Input::pointer_down(from));
    // One short of the threshold (straight-line): still a press.
    let short = DRAG_THRESHOLD.0 - 1.0;
    harness.send(Input::pointer_move(offset(from, 0.0, short)));
    harness.advance(Duration::from_millis(1));
    assert!(read(&mut harness, &DRAGS).is_empty(), "under the threshold");
    let crossed = offset(from, 0.0, DRAG_THRESHOLD.0);
    harness.send(Input::pointer_move(crossed));
    harness.send(Input::pointer_move(offset(from, 30.0, 3.0)));
    harness.send(Input::pointer_up(offset(from, 30.0, 3.0)));
    harness.advance(Duration::from_millis(1));
    assert_eq!(
        read(&mut harness, &DRAGS),
        vec![DragStart { from, at: crossed }],
        "once, at the crossing"
    );
    assert_eq!(read(&mut harness, &OPENED), 0, "a drag does not open");
    // A tap opens.
    harness.send(Input::click(from));
    harness.advance(Duration::from_millis(1));
    assert_eq!(read(&mut harness, &OPENED), 1);
    assert_eq!(read(&mut harness, &DRAGS).len(), 1);
}

#[test]
fn the_pointer_on_the_card_is_told_and_shows_the_actions() {
    let mut harness = rested();
    assert_eq!(
        harness.attr(".ds-shot", "data-hover").as_deref(),
        Some("off")
    );
    harness.send(Input::pointer_move(picture_centre(&harness)));
    harness.advance(Duration::from_millis(1));
    assert_eq!(read(&mut harness, &HOVERS), vec![Hover::Over]);
    assert_eq!(
        harness.attr(".ds-shot", "data-hover").as_deref(),
        Some("on")
    );
    let delete = harness
        .centre(".ds-shot-action .ds-button")
        .expect("the action is laid out");
    harness.send(Input::click(delete));
    harness.advance(Duration::from_millis(1));
    assert_eq!(read(&mut harness, &DELETED), 1);
    assert_eq!(read(&mut harness, &OPENED), 0, "an action does not open");
    harness.send(Input::pointer_move(Point {
        x: Px(390.0),
        y: Px(290.0),
    }));
    harness.advance(Duration::from_millis(1));
    assert_eq!(read(&mut harness, &HOVERS), vec![Hover::Over, Hover::Away]);
}

/// Drag from `from` by `dx` in four slow steps (100 ms apart, far under the fling speed), then
/// release there.
fn slow_drag(harness: &mut Harness, from: Point, dx: f32) -> Instant {
    harness.send(Input::pointer_move(from));
    harness.send(Input::pointer_down(from));
    for step in 1..=4u8 {
        harness.advance(Duration::from_millis(100));
        harness.send(Input::pointer_move(offset(
            from,
            dx * f32::from(step) / 4.0,
            0.0,
        )));
    }
    let released = harness.now();
    harness.send(Input::pointer_up(offset(from, dx, 0.0)));
    released
}

#[test]
fn beside_swipe_to_dismiss_a_drag_right_is_the_swipe_and_left_a_drag_out() {
    let mut harness = Harness::new(
        Swipeable,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    settle_until(&mut harness, |h| presence(h).as_deref() == Some("present"));
    let from = picture_centre(&harness);
    // Left: a drag out, which the swipe follows only damped and springs back from.
    slow_drag(&mut harness, from, -40.0);
    harness.advance(Duration::from_millis(1));
    assert_eq!(
        read(&mut harness, &DRAGS).len(),
        1,
        "a drag out to the left"
    );
    assert_eq!(read(&mut harness, &SWIPED), 0);
    // Right, past the dismiss distance: the swipe's.
    let flight = settle(Anim::PanelOut, MotionLevel::Standard);
    let released = slow_drag(&mut harness, from, 100.0);
    assert_eq!(
        harness.attr(".ds-shot", "data-swipe").as_deref(),
        Some("gone")
    );
    let reported = swiped_by(&mut harness, released + SETTLE_BOUND);
    assert_eq!(
        reported.duration_since(released),
        flight,
        "reported exactly when its flight settled"
    );
    assert_eq!(
        read(&mut harness, &DRAGS).len(),
        1,
        "no drag out to the right"
    );
    assert_eq!(read(&mut harness, &OPENED), 0, "neither gesture opens");
    // Hidden once it has flown, then shown again: a fresh card, not one stuck gone.
    harness.within(|| *SWIPE_SHOWN.write() = Shown::Hidden);
    settle_until(&mut harness, |h| presence(h).is_none());
    assert_eq!(read(&mut harness, &HIDDEN), 1);
    harness.within(|| *SWIPE_SHOWN.write() = Shown::Visible);
    harness.advance(Duration::from_millis(1));
    assert_eq!(presence(&harness).as_deref(), Some("entering"));
    assert_eq!(
        harness.attr(".ds-shot", "data-swipe").as_deref(),
        Some("rest")
    );
}

/// Advance in 1 ms steps until the swipe has been reported, up to `bound` on the harness's own
/// clock (`settle_until` reads only the document, and the report is a signal); the instant it
/// landed, exact to the millisecond.
fn swiped_by(harness: &mut Harness, bound: Instant) -> Instant {
    while harness.now() < bound {
        if read(harness, &SWIPED) == 1 {
            return harness.now();
        }
        harness.advance(Duration::from_millis(1));
    }
    panic!("the swipe was never reported");
}
