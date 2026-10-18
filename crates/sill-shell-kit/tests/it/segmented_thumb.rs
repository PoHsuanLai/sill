//! SegmentedControl's thumb and labels under real clicks (sill's control center, 2026-09-28), on
//! `Clock::Virtual` so every frame falls at an exact instant, at scale 1 and 2, light and dark,
//! every accent, in the three places the desktop draws one: the control center's compact
//! AppearancePicker, the full picker, and the small power-profile control (unequal words).
//!
//! 1. At rest the thumb, where it is painted (its slide applied), is exactly the selected
//!    segment's laid-out box; while it slides it is a segment wide and inside the track. The
//!    compact picker once made its segments a flex row sized by their words, so the thumb (a
//!    segment's equal share) missed the selected one; at scale 2 the thumb's own arithmetic also
//!    drifted from the rounded cells.
//! 2. On every frame, settled or mid-slide, after slow and rapid clicks, a label clear of the
//!    thumb reads at 4.5:1 against the track and a label wholly under it at 4.5:1 against the
//!    thumb (a label the thumb is crossing is neither). The labels' ink once ran a colour
//!    transition on its own clock: the picked label went to the thumb's light ink on the light
//!    track before the thumb arrived, and after a theme pick every label faded from the old
//!    scheme's ink (1.1:1 to 2.3:1 on the track).
//!
//! Also painted: at scale 2 the pixels just inside the selected segment's edges are the thumb's.

use crate::support::probe;

use dioxus::prelude::*;
use ds::base::time::FRAME_TICK;
use ds::components::controls::radio_group::Arrangement;
use ds::components::controls::segmented::Tracking;
use ds::prelude::*;
use ds::style::tokens::control_size::ControlSize;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Part, Srgba, Viewport};
use probe::{distance, rect};
use sill_shell_kit::prelude::*;
use std::cell::Cell;
use std::time::Duration;

thread_local! {
    static CASE: Cell<(Theme, Accent)> = const { Cell::new((Theme::Light, Accent::Blue)) };
}

/// The three controls, each group's selector.
const GROUPS: [&str; 3] = [
    ".compact .ds-segmented",
    ".full .ds-segmented",
    ".power .ds-segmented",
];

/// WCAG AA for text.
const LEGIBLE: f32 = 4.5;

/// A frame of the spring driver.
const FRAME: Duration = FRAME_TICK;

/// Frames that let a Quick spring come to rest from two segments away.
const SETTLE_FRAMES: usize = 40;

#[allow(non_snake_case)]
fn Desk() -> Element {
    let (theme, accent) = CASE.get();
    let mut appearance = use_signal(|| Appearance {
        theme,
        accent,
        ..Appearance::default()
    });
    let mut profile = use_signal(|| 1u8);
    let profiles = vec![
        (0, "Power Saver".to_owned()),
        (1, "Balanced".to_owned()),
        (2, "Performance".to_owned()),
    ];
    rsx! {
            Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: appearance(), material: Material::Popover,
                div { style: "width:320px",
                    ModuleGrid {
                        ModulePanel {
                            div { class: "compact",
    div { style: "display:grid;gap:14px",
                        SectionHeader { title: "Theme" }
                        SegmentedControl::<Theme> {
                            label: "Theme",
                            choices: Choice::pairs(Theme::ALL.iter().map(|theme| (*theme, theme.label()))),
                            tracking: Tracking::SelectOne(appearance().theme),
                            onchange: move |theme| appearance.set(Appearance { theme, ..appearance() }),
                        }
                        SectionHeader { title: "Accent" }
                        RadioGroup::<Accent> {
                            label: "Accent",
                            arrangement: Arrangement::Swatches,
                            choices: Accent::ALL.iter().map(|accent| Choice::accent(*accent)).collect(),
                            value: appearance().accent,
                            onchange: move |accent| appearance.set(Appearance { accent, ..appearance() }),
                        }
                    }
                            }
                        }
                        ModulePanel {
                            div { class: "power",
                                SegmentedControl::<u8> { label: "Power Mode", size: ControlSize::Mini, choices: Choice::pairs(profiles),
                                    tracking: Tracking::SelectOne(profile()), onchange: move |next| profile.set(next) }
                            }
                        }
                    }
                    div { class: "full",
    div { style: "display:grid;gap:14px",
                        SectionHeader { title: "Theme" }
                        SegmentedControl::<Theme> {
                            label: "Theme",
                            choices: Choice::pairs(Theme::ALL.iter().map(|theme| (*theme, theme.label()))),
                            tracking: Tracking::SelectOne(appearance().theme),
                            onchange: move |theme| appearance.set(Appearance { theme, ..appearance() }),
                        }
                        SectionHeader { title: "Accent" }
                        RadioGroup::<Accent> {
                            label: "Accent",
                            arrangement: Arrangement::Swatches,
                            choices: Accent::ALL.iter().map(|accent| Choice::accent(*accent)).collect(),
                            value: appearance().accent,
                            onchange: move |accent| appearance.set(Appearance { accent, ..appearance() }),
                        }
                    }
                    }
                }
            }
        }
}

fn desk(scale_percent: u16, theme: Theme, accent: Accent) -> Harness {
    CASE.set((theme, accent));
    let view = Viewport {
        width: 400,
        height: 600,
        scale_percent,
    };
    let mut harness = Harness::new(Desk, HarnessConfig::new(view).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(600));
    harness
}

fn segment(group: &str, index: usize) -> String {
    format!("{group} .ds-segmented-segment:nth-child({index})")
}

/// The 1-based index of the pressed segment.
fn selected(harness: &Harness, group: &str) -> usize {
    (1..=harness.count(&format!("{group} .ds-segmented-segment")))
        .find(|&i| harness.attr(&segment(group, i), "aria-checked").as_deref() == Some("true"))
        .unwrap_or_else(|| panic!("{group}: nothing pressed:\n{}", harness.html()))
}

fn thumb(harness: &Harness, group: &str) -> Rect {
    harness
        .painted_rect(&format!("{group} .ds-segmented-indicator"), Part::Element)
        .unwrap_or_else(|| panic!("{group} has no thumb"))
}

fn right(rect: Rect) -> f32 {
    rect.origin.x.0 + rect.size.width.0
}

/// How much of `seg`'s width `thumb` covers, 0 to 1.
fn overlap(thumb: Rect, seg: Rect) -> f32 {
    let covered = right(thumb).min(right(seg)) - thumb.origin.x.0.max(seg.origin.x.0);
    (covered / seg.size.width.0).clamp(0.0, 1.0)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// At rest: the painted thumb is the selected segment's box.
fn assert_covers(harness: &Harness, group: &str, case: &str) {
    let at = selected(harness, group);
    let seg = rect(harness, &segment(group, at));
    let thumb = thumb(harness, group);
    assert!(
        near(thumb.origin.x.0, seg.origin.x.0)
            && near(thumb.origin.y.0, seg.origin.y.0)
            && near(thumb.size.width.0, seg.size.width.0)
            && near(thumb.size.height.0, seg.size.height.0),
        "{case} {group}: thumb {thumb:?} is not segment {at} {seg:?}"
    );
}

/// On any frame: the thumb is a segment wide, inside the track.
fn assert_thumb_in_track(harness: &Harness, group: &str, case: &str) {
    let seg = rect(harness, &segment(group, selected(harness, group)));
    let track = rect(harness, group);
    let thumb = thumb(harness, group);
    assert!(
        near(thumb.size.width.0, seg.size.width.0),
        "{case} {group}: thumb {thumb:?}, segment {seg:?}"
    );
    // A spring may overshoot the last segment by a little; never past the track.
    assert!(
        thumb.origin.x.0 >= track.origin.x.0 - 0.01 && right(thumb) <= right(track) + 0.01,
        "{case} {group}: thumb {thumb:?} leaves the track {track:?}"
    );
}

/// On any frame: every label clear of the thumb reads on the track, every label wholly under it
/// reads on the thumb.
fn assert_legible(harness: &Harness, group: &str, case: &str) {
    let track = harness.fill_of(group, Part::Element).expect("track fill");
    assert!(
        track.0[3] > 0.99,
        "{case} {group}: the track is not opaque: {track:?}"
    );
    let fill = harness
        .fill_of(&format!("{group} .ds-segmented-indicator"), Part::Element)
        .expect("thumb fill");
    let thumb = thumb(harness, group);
    for i in 1..=harness.count(&format!("{group} .ds-segmented-segment")) {
        let label = segment(group, i);
        let ink = harness.ink_of(&label).expect("label ink");
        let covered = overlap(thumb, rect(harness, &label));
        let ground: Srgba = match covered {
            c if c <= 0.0 => track,
            c if c >= 0.999 => fill,
            _ => continue,
        };
        let ratio = ink.over(ground).contrast(ground);
        assert!(
            ratio >= LEGIBLE,
            "{case} {group}: label {i} {:?} (covered {covered:.2}) is {ratio:.2}:1: ink {ink:?} on {ground:?}",
            harness.text_of(&label)
        );
    }
}

/// Every frame for `frames` frames: thumb in the track and every label legible.
fn watch(harness: &mut Harness, group: &str, frames: usize, case: &str) {
    for frame in 0..=frames {
        let case = format!("{case} frame {frame}");
        assert_thumb_in_track(harness, group, &case);
        assert_legible(harness, group, &case);
        harness.advance(FRAME);
    }
}

fn click(harness: &mut Harness, group: &str, index: usize) {
    let at = harness
        .centre(&segment(group, index))
        .unwrap_or_else(|| panic!("{group}: no segment {index}"));
    harness.send(Input::click(at));
}

/// Slow picks, each watched to rest, then a burst of picks a frame or two apart.
fn exercise(harness: &mut Harness, group: &str, case: &str) {
    assert_covers(harness, group, &format!("{case} at first"));
    for index in [3, 1, 2] {
        click(harness, group, index);
        let case = format!("{case} after picking {index}");
        watch(harness, group, SETTLE_FRAMES, &case);
        assert_eq!(selected(harness, group), index, "{case} {group}");
        assert_covers(harness, group, &case);
    }
    for (index, gap) in [(3, 1), (1, 2), (2, 1)] {
        click(harness, group, index);
        watch(harness, group, gap, &format!("{case} rapid {index}"));
    }
    click(harness, group, 3);
    let case = format!("{case} after the burst");
    watch(harness, group, SETTLE_FRAMES, &case);
    assert_eq!(selected(harness, group), 3, "{case} {group}");
    assert_covers(harness, group, &case);
}

fn run(scale_percent: u16, theme: Theme) {
    for accent in Accent::ALL.iter().copied() {
        for group in GROUPS {
            let mut harness = desk(scale_percent, theme, accent);
            let case = format!("{scale_percent}% {theme:?} {accent:?}");
            exercise(&mut harness, group, &case);
        }
    }
}

#[test]
fn scale_1_light() {
    run(100, Theme::Light);
}

#[test]
fn scale_1_dark() {
    run(100, Theme::Dark);
}

#[test]
fn scale_2_light() {
    run(200, Theme::Light);
}

#[test]
fn scale_2_dark() {
    run(200, Theme::Dark);
}

/// The pixel at CSS x `x`, at `seg`'s middle height, at `scale`.
fn pixel(frame: &image::RgbaImage, seg: Rect, x: f32, scale: f32) -> [u8; 4] {
    let y = seg.origin.y.0 + seg.size.height.0 / 2.0;
    frame.get_pixel((x * scale) as u32, (y * scale) as u32).0
}

fn bytes(colour: Srgba) -> [u8; 4] {
    colour.0.map(|c| (c * 255.0).round() as u8)
}

#[test]
fn at_scale_2_the_painted_thumb_fills_the_selected_segment_to_its_edges() {
    for theme in [Theme::Light, Theme::Dark] {
        for group in GROUPS {
            let mut harness = desk(200, theme, Accent::Blue);
            click(&mut harness, group, 3);
            harness.advance(Duration::from_millis(800));
            let frame = harness.render().expect("render");
            let seg = rect(&harness, &segment(group, 3));
            let thumb = bytes(
                harness
                    .fill_of(&format!("{group} .ds-segmented-indicator"), Part::Element)
                    .expect("thumb fill"),
            );
            let track = pixel(&frame, seg, seg.origin.x.0 - 1.0, 2.0);
            assert!(
                // The Mac's thumb is a raised white (light) or a lighter grey (dark) on a grey
                // well, a step of 20 or more, not the old ink-on-paper contrast.
                distance(thumb, track) > 20,
                "{theme:?} {group}: thumb {thumb:?}, track {track:?}"
            );
            for x in [seg.origin.x.0 + 1.0, right(seg) - 1.0] {
                let seen = pixel(&frame, seg, x, 2.0);
                assert!(
                    distance(seen, thumb) < 24,
                    "{theme:?} {group}: at x {x} (segment {seg:?}) the paint is {seen:?}, not the thumb's {thumb:?}"
                );
            }
        }
    }
}
