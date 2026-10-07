//! Hover under a resting pointer: Blitz's resolve re-hit-tests the last pointer
//! position against the new layout and records the new hovered element without events, so an
//! element that slides in under a pointer that is not moving never heard `pointerenter`, and
//! what the pointer left never heard `pointerleave`, until the pointer went out and back. The
//! harness's frame now dispatches that change (`blitz_kit::hover`).

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_shell_kit::notifications::banner_stack::{Banner, BannerKey};
use sill_shell_kit::notifications::parts::AppMark;
use sill_shell_kit::prelude::*;
use std::cell::RefCell;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 400,
    height: 400,
    scale_percent: 100,
};

/// Whether the block has slid over the resting pointer.
static SLID: GlobalSignal<Slide> = Signal::global(|| Slide::Away);
thread_local! {
    /// Every enter and leave, in order, as `"enter field"`: per thread, as each test runs its
    /// harness (and so its handlers) on its own.
    static HEARD: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slide {
    Away,
    /// Over the field's corner, where the pointer rests.
    Under,
    /// Over the whole field, so no point of it can be hit.
    Cover,
}

fn heard(what: &str, whom: &str) {
    HEARD.with(|heard| heard.borrow_mut().push(format!("{what} {whom}")));
}

/// A page holding a field and, beside it, an absolutely positioned block that can slide over
/// the field's top-left corner.
#[allow(non_snake_case)]
fn Slider() -> Element {
    let (left, side) = match SLID() {
        Slide::Away => (320, 60),
        Slide::Under => (0, 60),
        Slide::Cover => (0, 300),
    };
    rsx! {
        div {
            class: "page",
            style: "position: relative; width: 400px; height: 400px; margin: 0;",
            onpointerenter: |_| heard("enter", "page"),
            onpointerleave: |_| heard("leave", "page"),
            div {
                class: "field",
                style: "position: absolute; left: 0; top: 0; width: 300px; height: 300px;",
                onpointerenter: |_| heard("enter", "field"),
                onpointerleave: |_| heard("leave", "field"),
            }
            div {
                class: "block",
                style: "position: absolute; left: {left}px; top: 0; width: {side}px; height: {side}px;",
                onpointerenter: |_| heard("enter", "block"),
                onpointerleave: |_| heard("leave", "block"),
            }
        }
    }
}

fn count(what: &str) -> usize {
    HEARD.with(|heard| heard.borrow().iter().filter(|line| *line == what).count())
}

fn all_heard() -> Vec<String> {
    HEARD.with(|heard| heard.borrow().clone())
}

fn slide(harness: &mut Harness, to: Slide) {
    harness.within(|| *SLID.write() = to);
    harness.advance(Duration::from_millis(1));
}

const REST: Point = Point {
    x: Px(20.0),
    y: Px(20.0),
};

#[test]
fn a_block_sliding_under_a_resting_pointer_is_entered_and_the_field_left() {
    let mut harness = Harness::new(Slider, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.send(Input::pointer_move(REST));
    assert_eq!(count("enter field"), 1);
    slide(&mut harness, Slide::Under);
    harness.send(Input::pointer_move(Point {
        x: Px(REST.x.0 + 1.0),
        y: REST.y,
    }));
    settle_until(&mut harness, |_| {
        count("enter block") == 1 && count("leave field") == 1
    });
    harness.advance(Duration::from_millis(20));
    let heard = all_heard();
    assert_eq!(
        heard,
        ["enter page", "enter field", "leave field", "enter block"],
        "one enter each, the page (shared by both) never re-entered"
    );
}

#[test]
fn it_is_entered_even_before_the_pointer_moves_again() {
    let mut harness = Harness::new(Slider, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.send(Input::pointer_move(REST));
    slide(&mut harness, Slide::Under);
    settle_until(&mut harness, |_| count("enter block") == 1);
    slide(&mut harness, Slide::Away);
    settle_until(&mut harness, |_| {
        count("leave block") == 1 && count("enter field") == 2
    });
    assert_eq!(count("enter page"), 1);
    assert_eq!(count("leave page"), 0);
}

#[test]
fn a_block_sliding_in_before_any_pointer_hears_nothing() {
    let mut harness = Harness::new(Slider, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    slide(&mut harness, Slide::Under);
    harness.advance(Duration::from_millis(20));
    assert_eq!(all_heard().len(), 0);
}

/// The field covered whole cannot be hovered back without events (Blitz can only be told to
/// hover a point), so the host clears the hover before replaying: the block is still entered,
/// though the field's leave is lost and the page is entered a second time.
#[test]
fn a_block_covering_the_whole_field_is_still_entered() {
    let mut harness = Harness::new(Slider, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.send(Input::pointer_move(REST));
    slide(&mut harness, Slide::Cover);
    settle_until(&mut harness, |_| count("enter block") == 1);
    // The limitation, pinned so that lifting it shows up here.
    assert_eq!(count("leave field"), 0);
    assert_eq!(count("enter page"), 2);
}

/// The banners shown, by key.
static SHOWN: GlobalSignal<Vec<u32>> = Signal::global(Vec::new);

#[allow(non_snake_case)]
fn Stack() -> Element {
    let banners: Vec<Banner> = SHOWN()
        .into_iter()
        .map(|n| Banner {
            key: BannerKey(n),
            card: rsx! {
                NotificationCard {
                    app: AppMark { icon: IconSource::Glyph(Icon::Mail), name: "Mail".into() },
                    age: "now",
                    summary: "Banner {n}",
                    body: "Two lines of body, or thereabouts, to give the banner some height.",
                    on_close: |_| {},
                    on_open: |_| {},
                }
            },
        })
        .collect();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Toast,
            BannerStack { banners }
        }
    }
}

const CARD: &str = ".ds-notification";

fn show(harness: &mut Harness, keys: &[u32]) {
    harness.within(|| *SHOWN.write() = keys.to_vec());
    harness.advance(Duration::from_millis(1));
}

fn at_rest(harness: &Harness) -> bool {
    harness
        .attr(".ds-banner", "data-presence")
        .is_some_and(|presence| presence == "present")
}

#[test]
fn a_banner_arriving_under_a_resting_pointer_is_hovered() {
    let mut harness = Harness::new(Stack, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    // Where a banner comes to rest: shown once, measured, and taken away again.
    show(&mut harness, &[1]);
    settle_until(&mut harness, at_rest);
    let there = harness
        .centre(".ds-notification-plate")
        .expect("a banner plate");
    show(&mut harness, &[]);
    settle_until(&mut harness, |h| h.count(".ds-banner") == 0);

    harness.send(Input::pointer_move(there));
    show(&mut harness, &[2]);
    settle_until(&mut harness, at_rest);
    harness.send(Input::pointer_move(Point {
        x: Px(there.x.0 + 1.0),
        y: there.y,
    }));
    settle_until(&mut harness, |h| {
        h.attr(CARD, "data-hover").as_deref() == Some("on")
    });
}
