//! design/30 section 2.10 and design/25-EMOJI.md section 5 on a real Blitz document: an animated
//! emoji plays the asset's own animation once through when it appears, at the asset's frame
//! durations, then rests on frame 0 with nothing scheduled or painting (the idle-frame rule); a
//! new wake stamp plays it once more; under Reduced motion, or with `EmojiPlayback::Still`, the
//! frame never leaves 0.

use dioxus::prelude::*;
use ds::motion::wake::WakeStamp;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::emoji::disc::EmojiPlayback;
use sill_shell_kit::emoji::id::EmojiId;
use sill_shell_kit::prelude::*;
use sill_shell_kit::user_picture::size::PictureSize;
use std::collections::BTreeSet;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 200,
    height: 200,
    scale_percent: 100,
};

/// Wink's loop is shorter than this (manifest.json).
const LOOP: Duration = Duration::from_secs(4);

static WAKE: GlobalSignal<WakeStamp> = Signal::global(WakeStamp::default);
static MOTION: GlobalSignal<Motion> = Signal::global(|| Motion::Standard);

#[allow(non_snake_case)]
fn Stage() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { theme: Theme::Light, motion: MOTION(), ..Appearance::default() }, material: Material::Window,
            AnimatedEmoji { emoji: EmojiId::Wink, size: PictureSize::Large, wake: WAKE() }
        }
    }
}

#[allow(non_snake_case)]
fn StillStage() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { theme: Theme::Light, ..Appearance::default() }, material: Material::Window,
            AnimatedEmoji { emoji: EmojiId::Wink, size: PictureSize::Medium, playback: EmojiPlayback::Still }
        }
    }
}

fn frame(harness: &Harness) -> Option<String> {
    harness.attr(".ds-emoji-face", "data-frame")
}

fn moving(harness: &Harness) -> bool {
    frame(harness).as_deref() != Some("0")
}

/// Pixels of the render that are not the window's plain ground: the emoji's own paint.
fn inked(image: &image::RgbaImage) -> usize {
    let ground = *image.get_pixel(2, 2);
    image.pixels().filter(|pixel| **pixel != ground).count()
}

/// The distinct frames seen over `span`, sampled every 50 ms.
fn frames_over(harness: &mut Harness, span: Duration) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    for _ in 0..(span.as_millis() / 50) {
        harness.advance(Duration::from_millis(50));
        seen.extend(frame(harness));
    }
    seen
}

#[test]
fn an_emoji_plays_its_animation_once_and_rests_on_frame_zero() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    let appeared = harness.now();
    let before = harness.render().expect("render");
    assert!(inked(&before) > 2000, "the emoji is not painted");
    let moved = settle_until(&mut harness, moving);
    assert!(
        moved.duration_since(appeared) < Duration::from_secs(1),
        "the first frame advanced only after {:?}",
        moved.duration_since(appeared)
    );
    let seen = frames_over(&mut harness, Duration::from_secs(1));
    assert!(seen.len() >= 5, "only frames {seen:?} in a second");
    let ended = settle_until(&mut harness, |h| !moving(h));
    assert!(
        ended.duration_since(appeared) < LOOP,
        "one pass took {:?}",
        ended.duration_since(appeared)
    );
    // At rest for good: no frame moves and nothing asks for one.
    assert!(!harness.is_animating(), "asks for frames at rest");
    for _ in 0..40 {
        harness.advance(Duration::from_millis(250));
        assert_eq!(frame(&harness).as_deref(), Some("0"), "it looped");
    }
    assert!(!harness.is_animating());
}

#[test]
fn a_new_wake_stamp_plays_it_once_more() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    settle_until(&mut harness, moving);
    settle_until(&mut harness, |h| !moving(h));
    harness.within(|| {
        let next = WAKE.peek().next();
        *WAKE.write() = next;
    });
    settle_until(&mut harness, moving);
    settle_until(&mut harness, |h| !moving(h));
    assert!(!harness.is_animating());
}

#[test]
fn under_reduced_motion_only_the_still_frame_shows() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| *MOTION.write() = Motion::Reduced);
    harness.advance(Duration::from_millis(100));
    harness.within(|| {
        let next = WAKE.peek().next();
        *WAKE.write() = next;
    });
    for _ in 0..10 {
        harness.advance(Duration::from_millis(100));
        assert_eq!(
            frame(&harness).as_deref(),
            Some("0"),
            "a frame under Reduced"
        );
    }
    harness.within(|| *MOTION.write() = Motion::Standard);
}

#[test]
fn a_still_picture_never_leaves_its_rest_frame() {
    let mut harness = Harness::new(
        StillStage,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    for _ in 0..10 {
        harness.advance(Duration::from_millis(100));
        assert_eq!(
            frame(&harness).as_deref(),
            Some("0"),
            "a still picture moved"
        );
    }
    assert!(!harness.is_animating());
}
