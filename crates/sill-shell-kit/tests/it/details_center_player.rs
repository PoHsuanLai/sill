//! design/26 on a real Blitz document, on the virtual clock: Now Playing. Play/pause offers
//! the next action, cross-faded; the position steps once a second while playing and costs 0
//! frames paused; a new track cross-fades its art and titles, a restated one plays nothing;
//! Reduced (R7).

use dioxus::prelude::*;
use ds::components::controls::button_model::{Bezel, IconSwap, ImagePosition};
use ds::prelude::*;
use ds::style::tokens::control_size::ControlSize;
use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{ClassPresence, Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::now_playing::kind::Playback;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 360,
    height: 160,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

static PLAYBACK: GlobalSignal<Playback> = Signal::global(|| Playback::Paused);
static TITLE: GlobalSignal<&'static str> = Signal::global(|| "Clair de lune");
static AT: GlobalSignal<Duration> = Signal::global(|| Duration::from_millis(61_400));
static MOTION: GlobalSignal<Motion> = Signal::global(|| Motion::Standard);

#[allow(non_snake_case)]
fn Player() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: MOTION(), ..Appearance::default() }, material: Material::Window,
            div { style: "width:340px",
                div { id: "track", NowPlayingTrack { title: TextLine::from(TITLE()), by: Some(TextLine::from("Claude Debussy")) } }
                div { id: "toggle",
                    Button {
                        bezel: Bezel::Toolbar,
                        size: ControlSize::Large,
                        image: ImagePosition::Only,
                        swap: IconSwap::CrossFade,
                        icon: PLAYBACK().next_action(),
                        label: PLAYBACK().label(),
                        onclick: move |_| *PLAYBACK.write() = match PLAYBACK() {
                            Playback::Paused => Playback::Playing,
                            Playback::Playing | Playback::Buffering(_) => Playback::Paused,
                        },
                    }
                }
                div { id: "position", TrackPosition { at: AT(), length: Duration::from_secs(200), playback: PLAYBACK() } }
            }
        }
    }
}

fn set(harness: &mut Harness, playback: Playback) {
    harness.within(|| *PLAYBACK.write() = playback);
}

fn incoming(harness: &Harness) -> String {
    harness
        .attr("#toggle [*|data-morph=in]", "class")
        .unwrap_or_default()
}

#[test]
fn play_pause_offers_the_next_action_cross_faded() {
    let mut harness = virtual_harness(Player);
    assert_eq!(
        harness.attr("#toggle button", "aria-label").as_deref(),
        Some("Play")
    );
    assert_settles_to_zero_frames(&mut harness);
    set(&mut harness, Playback::Playing);
    harness.advance(ms(0));
    assert_eq!(
        harness.attr("#toggle button", "aria-label").as_deref(),
        Some("Pause")
    );
    let class = incoming(&harness);
    assert!(class.contains("a-morph-fade-in"), "{class}");
    assert_eq!(
        harness.count("#toggle [*|data-morph=out]"),
        1,
        "the old glyph fades out over the new"
    );
    // Not `assert_settles_to_zero_frames`: the track is still `Playing` here (61.4 s into 200 s),
    // so `TrackPosition`'s own per-second ticker (`track_position.rs`) keeps a Rust timer
    // legitimately pending until the track ends or it is paused, the documented R3 exception for
    // a live position/clock display (design/26-DETAILS.md R3, "Clock second hand"). Check only
    // the half this moment is really about: the glyph's own CSS fade runs its course and stops.
    settle_until(&mut harness, |h| !h.is_animating());
    assert!(
        !harness.is_animating(),
        "the glyph's own transition should be done: {}",
        harness.html()
    );
    let at = harness.centre("#toggle button").expect("the button");
    harness.send(Input::click(at));
    harness.advance(ms(0));
    assert_eq!(harness.within(|| *PLAYBACK.read()), Playback::Paused);
    assert_eq!(
        harness.attr("#toggle button", "aria-label").as_deref(),
        Some("Play")
    );
    assert_settles_to_zero_frames(&mut harness);
}

fn elapsed(harness: &Harness) -> Option<String> {
    harness.text_of("#position .ds-track-times .ds-label")
}

#[test]
fn the_position_steps_on_the_second_while_playing_and_holds_paused() {
    let mut harness = virtual_harness(Player);
    assert_eq!(elapsed(&harness).as_deref(), Some("1:01"));
    assert_settles_to_zero_frames(&mut harness);
    set(&mut harness, Playback::Playing);
    harness.advance(ms(0));
    // The first step lands on the next whole second (61.4 s + 600 ms), not a second later.
    harness.advance(ms(599));
    assert_eq!(elapsed(&harness).as_deref(), Some("1:01"));
    harness.advance(ms(1));
    assert_eq!(elapsed(&harness).as_deref(), Some("1:02"));
    // The bar follows each whole second over --t-move (design/30 section 1.3), then rests:
    // between one second's step and the next nothing wakes.
    harness.advance(ms(3_000));
    assert_eq!(elapsed(&harness).as_deref(), Some("1:05"));
    harness.advance(ms(400));
    let quiet = harness.wakes();
    harness.advance(ms(300));
    assert_eq!(
        harness.wakes(),
        quiet,
        "the bar rests between one second's step and the next"
    );
    assert_eq!(elapsed(&harness).as_deref(), Some("1:05"));
    let bar = harness
        .attr("#position .ds-progress-fill", "style")
        .unwrap_or_default();
    assert!(bar.contains("--f:0.325"), "{bar}");
    // A new report re-anchors; paused, it holds at 0 frames.
    harness.within(|| *AT.write() = Duration::from_secs(120));
    set(&mut harness, Playback::Paused);
    harness.advance(ms(0));
    assert_eq!(elapsed(&harness).as_deref(), Some("2:00"));
    assert_settles_to_zero_frames(&mut harness);
    harness.advance(ms(5_000));
    assert_eq!(elapsed(&harness).as_deref(), Some("2:00"));
}

#[test]
fn a_new_track_cross_fades_and_a_restated_one_plays_nothing() {
    let mut harness = virtual_harness(Player);
    harness.within(|| *TITLE.write() = "Clair de lune");
    harness.advance(ms(0));
    assert_eq!(
        harness.count("#track [*|data-morph=out]"),
        0,
        "the same track is no moment"
    );
    assert_settles_to_zero_frames(&mut harness);
    harness.within(|| *TITLE.write() = "Gymnopédie No. 1");
    harness.advance(ms(0));
    assert_eq!(
        harness.count("#track .ds-track-words [*|data-morph=out]"),
        1
    );
    assert_eq!(harness.count("#track .ds-track-art [*|data-morph=out]"), 1);
    assert_eq!(
        harness.has_class(
            "#track .ds-track-words [*|data-morph=in]",
            "a-morph-fade-in"
        ),
        ClassPresence::Present
    );
    assert_eq!(
        harness
            .text_of("#track .ds-track-words [*|data-morph=out] .ds-track-title")
            .as_deref(),
        Some("Clair de lune")
    );
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(harness.count("#track [*|data-morph=out]"), 0);
    assert_eq!(
        harness.text_of("#track .ds-track-title").as_deref(),
        Some("Gymnopédie No. 1")
    );
}

#[test]
fn reduced_snaps_the_glyph_and_the_track() {
    let mut harness = virtual_harness(Player);
    harness.within(|| *MOTION.write() = Motion::Reduced);
    harness.advance(ms(20));
    set(&mut harness, Playback::Playing);
    harness.within(|| *TITLE.write() = "Trois Gnossiennes");
    harness.advance(ms(0));
    assert_eq!(
        harness.count("#toggle [*|data-morph=out]"),
        0,
        "the glyph snaps"
    );
    assert_eq!(
        harness.count("#track [*|data-morph=out]"),
        0,
        "the track snaps"
    );
}
