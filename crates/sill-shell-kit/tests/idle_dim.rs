//! IdleDim's fade on a real Blitz document (design/04-COMPONENTS.md section 53;
//! design/22-SETTINGS.md section 3.24; CONSUMING.md "Idle dim"): dimming fades in over
//! `--t-idle-dim`, waking always snaps even mid-fade, a live settings edit never animates, and
//! Reduced motion shows the level at once. Every case ends at 0 frames (R3).

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::idle_dim::{IdleDim, IdleDimPhase};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 240,
    height: 160,
    scale_percent: 100,
};

static LEVEL: GlobalSignal<Percent> = Signal::global(|| Percent(60));
static PHASE: GlobalSignal<IdleDimPhase> = Signal::global(|| IdleDimPhase::Awake);
static MOTION: GlobalSignal<Motion> = Signal::global(|| Motion::Standard);

#[allow(non_snake_case)]
fn Overlay() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: MOTION(), ..Appearance::default() }, material: Material::Window,
            OverlayBody {}
        }
    }
}

#[allow(non_snake_case)]
#[component]
fn OverlayBody() -> Element {
    rsx! {
        IdleDim { level: LEVEL(), phase: PHASE() }
    }
}

/// The overlay's opacity in permille, read from the style the component draws.
fn read(harness: &Harness, selector: &str) -> i64 {
    harness
        .attr(selector, "style")
        .and_then(|style| style.strip_prefix("opacity:")?.trim().parse::<f32>().ok())
        .map_or(-1, |opacity| (opacity * 1000.0).round() as i64)
}

fn virtual_harness() -> Harness {
    Harness::new(Overlay, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

#[test]
fn dimming_fades_in_over_t_idle_dim_and_lands_on_the_level() {
    let mut harness = virtual_harness();
    assert_eq!(read(&harness, ".ds-idle-dim"), 0);
    harness.within(|| *PHASE.write() = IdleDimPhase::Dimmed);
    // 600 permille (Percent(60)) is the target; --t-idle-dim is 2000ms. On the virtual clock
    // `advance` is exact (CONSUMING.md "the virtual clock"), so a fixed instant well short of
    // the fade's length is a real assertion, not a wall-clock race.
    harness.advance(Duration::from_millis(900));
    let mid = read(&harness, ".ds-idle-dim");
    assert!((1..600).contains(&mid), "expected a mid-fade value: {mid}");
    harness.advance(Duration::from_secs(2));
    assert_eq!(read(&harness, ".ds-idle-dim"), 600);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn waking_snaps_to_zero_even_mid_fade() {
    let mut harness = virtual_harness();
    harness.within(|| *PHASE.write() = IdleDimPhase::Dimmed);
    harness.advance(Duration::from_millis(900));
    let mid = read(&harness, ".ds-idle-dim");
    assert!((1..600).contains(&mid), "expected a mid-fade value: {mid}");
    harness.within(|| *PHASE.write() = IdleDimPhase::Awake);
    // No advance at all: the very next frame already reads zero.
    harness.advance(Duration::ZERO);
    assert_eq!(read(&harness, ".ds-idle-dim"), 0);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn a_settings_edit_while_dimmed_never_animates() {
    let mut harness = virtual_harness();
    harness.within(|| *PHASE.write() = IdleDimPhase::Dimmed);
    harness.advance(Duration::from_secs(3));
    assert_eq!(read(&harness, ".ds-idle-dim"), 600);
    harness.within(|| *LEVEL.write() = Percent(80));
    // A settings change repaints with the new value on its next frame; it does not fade
    // (design/22-SETTINGS.md section 2).
    harness.advance(Duration::ZERO);
    assert_eq!(read(&harness, ".ds-idle-dim"), 800);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn reduced_motion_shows_the_level_at_once_with_no_fade() {
    let mut harness = virtual_harness();
    harness.within(|| *MOTION.write() = Motion::Reduced);
    harness.within(|| *PHASE.write() = IdleDimPhase::Dimmed);
    harness.advance(Duration::ZERO);
    assert_eq!(read(&harness, ".ds-idle-dim"), 600);
    assert_settles_to_zero_frames(&mut harness);
}
