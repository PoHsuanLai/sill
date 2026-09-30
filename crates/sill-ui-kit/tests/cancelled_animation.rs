//! A CSS animation whose `animation-name` goes away before the
//! animation has ended leaves its last animated value on the element for good, in paint and in
//! the hit test, while `Harness::rect` (layout, no transform) reads the resting box. No wall
//! clock is involved: this reproduces on every run. Ignored until the Blitz pin restyles an
//! element whose animation set changed (stylo's `ElementAnimationSet::dirty`, which Blitz's
//! `resolve_stylist` never reads).

use dioxus::prelude::*;
use ds::{Point, Px};
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use std::time::Duration;

const CSS: &str = "@keyframes slide{ from{ transform:translateX(400px); } to{ transform:none; } }
.box{ position:absolute; left:0; top:0; width:100px; height:100px; background:red; }
.box.on{ animation:slide 1000ms linear; }
.btn{ position:absolute; left:600px; top:0; width:50px; height:50px; }";

/// A box sliding in, and a button that takes its animation away.
fn app() -> Element {
    let mut on = use_signal(|| true);
    rsx! {
        style { {CSS} }
        div { class: if on() { "box on" } else { "box" } }
        div { class: "btn", onpointerdown: move |_| on.set(false) }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 800,
    height: 200,
    scale_percent: 100,
};

fn at(x: f32, y: f32) -> Point {
    Point { x: Px(x), y: Px(y) }
}

/// Take the animation away, `after` into it, then let two seconds pass.
fn cancelled(after: u64) -> Harness {
    let mut h = Harness::new(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    h.advance(Duration::from_millis(after));
    h.send(Input::click(at(620.0, 20.0)));
    h.advance(Duration::from_millis(2000));
    assert_eq!(h.attr(".box", "class").as_deref(), Some("box"));
    assert!(!h.is_animating());
    h
}

#[test]
fn an_animation_removed_after_its_end_leaves_nothing() {
    let h = cancelled(1100);
    assert!(h.hits(at(50.0, 50.0), ".box"));
}

#[test]
fn an_animation_removed_midway_leaves_nothing() {
    let mut h = cancelled(20);
    let rect = h.rect(".box").expect("the box");
    assert!(rect.origin.x.0 < 10.0, "layout says at rest: {rect:?}");
    let pixel = h.render().expect("paint").get_pixel(50, 50).0;
    assert_eq!(pixel, [255, 0, 0, 255], "painted where it rests");
    assert!(h.hits(at(50.0, 50.0), ".box"), "hit where it rests");
}
