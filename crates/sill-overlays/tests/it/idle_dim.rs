//! The idle dim surface in a `ds_harness::Harness`, driven only by the idle service's published
//! `Idle.dim` through the
//! fake services' feed: at rest awake it draws a clear scrim and asks for no frames; a dim fades
//! in over `--t-idle-dim` to `100 - the dim level`; a wake snaps it out even mid-fade; a
//! level edit while dimmed does not animate; Reduced motion shows the level at once. On the
//! virtual clock with the appearance pinned, and every change ends at 0 frames.

use ds::prelude::*;
use std::time::Duration;

use ds_harness::Viewport;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_settings::Environment;
use sill_model::idle::{DimLevel, DimShown, Idle, IdleMode, IdlePhase};
use sill_overlays::IdleDim;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::hooks::PinnedAppearance;

const VIEW: Viewport = Viewport {
    width: 320,
    height: 200,
    scale_percent: 100,
};

const SCRIM: &str = ".ds-idle-dim";

type Running = SurfaceHarness<IdleDim>;

fn start(appearance: PinnedAppearance) -> Running {
    let mut run = SurfaceHarness::<IdleDim>::setup((), ())
        .viewport(VIEW)
        .appearance(appearance)
        .start();
    run.harness.advance(Duration::ZERO);
    assert_eq!(run.harness.count(SCRIM), 1, "one scrim, mounted at once");
    run
}

fn reduced() -> PinnedAppearance {
    let mut environment = Environment::default();
    environment.settings.appearance.motion_level = Motion::Reduced;
    PinnedAppearance(environment)
}

/// The idle service publishes `dim`, and the next frame runs.
fn publish(run: &mut Running, dim: DimShown) {
    let phase = match dim {
        DimShown::Clear => IdlePhase::Active,
        DimShown::Dimmed(_) => IdlePhase::Dimmed,
    };
    run.feeds.idle.send_replace(Idle {
        mode: IdleMode::Own,
        phase,
        dim,
        ..Idle::default()
    });
    run.harness.advance(Duration::ZERO);
    run.harness.advance(Duration::ZERO);
}

/// The scrim's opacity in thousandths, from its inline style.
fn scrim(run: &Running) -> u32 {
    let style = run.harness.attr(SCRIM, "style").unwrap_or_default();
    let value = style
        .split(';')
        .find_map(|part| part.trim().strip_prefix("opacity:"))
        .unwrap_or_else(|| panic!("no opacity in {style:?}"));
    let opacity: f32 = value
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("{value:?}: {e}"));
    (opacity * 1000.0).round() as u32
}

fn advance(run: &mut Running, ms: u64) {
    run.harness.advance(Duration::from_millis(ms));
}

#[test]
fn awake_at_rest_the_scrim_is_clear_and_draws_no_frames() {
    let mut run = start(PinnedAppearance::default());
    assert_eq!(scrim(&run), 0);
    assert_settles_to_zero_frames(&mut run.harness);
    publish(&mut run, DimShown::Clear);
    assert_eq!(scrim(&run), 0, "a Clear while clear changes nothing");
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_dim_fades_in_over_t_idle_dim_to_what_the_display_does_not_keep() {
    let mut run = start(PinnedAppearance::default());
    // the dim level 30: the display keeps 30 %, the scrim covers 70 %.
    publish(&mut run, DimShown::Dimmed(DimLevel(30)));
    advance(&mut run, 900);
    let mid = scrim(&run);
    assert!((1..700).contains(&mid), "mid-fade at 900 ms: {mid}");
    // `--t-idle-dim` is 2000 ms.
    advance(&mut run, 1_200);
    assert_eq!(scrim(&run), 700);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_wake_snaps_the_scrim_out_even_mid_fade() {
    let mut run = start(PinnedAppearance::default());
    publish(&mut run, DimShown::Dimmed(DimLevel(50)));
    advance(&mut run, 900);
    let mid = scrim(&run);
    assert!((1..500).contains(&mid), "mid-fade at 900 ms: {mid}");
    publish(&mut run, DimShown::Clear);
    assert_eq!(scrim(&run), 0, "the next frame is clear");
    assert_settles_to_zero_frames(&mut run.harness);

    // And from a settled dim.
    publish(&mut run, DimShown::Dimmed(DimLevel(50)));
    advance(&mut run, 2_500);
    assert_eq!(scrim(&run), 500);
    assert_settles_to_zero_frames(&mut run.harness);
    publish(&mut run, DimShown::Clear);
    assert_eq!(scrim(&run), 0);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_level_edit_while_dimmed_does_not_animate() {
    let mut run = start(PinnedAppearance::default());
    publish(&mut run, DimShown::Dimmed(DimLevel(50)));
    advance(&mut run, 2_500);
    assert_eq!(scrim(&run), 500);
    assert_settles_to_zero_frames(&mut run.harness);
    publish(&mut run, DimShown::Dimmed(DimLevel(20)));
    assert_eq!(scrim(&run), 800, "snapped to the new level");
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn reduced_motion_dims_at_once() {
    let mut run = start(reduced());
    publish(&mut run, DimShown::Dimmed(DimLevel(50)));
    assert_eq!(scrim(&run), 500, "no fade at all");
    assert_settles_to_zero_frames(&mut run.harness);
    publish(&mut run, DimShown::Clear);
    assert_eq!(scrim(&run), 0);
    assert_settles_to_zero_frames(&mut run.harness);
}
