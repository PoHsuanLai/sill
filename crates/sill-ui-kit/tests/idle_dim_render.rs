//! The idle dim surface in a `ds_native::Harness`, driven only by the idle service's published
//! `Idle.dim` through the
//! fake services' feed: at rest awake it draws a clear scrim and asks for no frames; a dim fades
//! in over `--t-idle-dim` to `100 - idle.dim_level_pct`; a wake snaps it out even mid-fade; a
//! level edit while dimmed does not animate; Reduced motion shows the level at once. On the
//! virtual clock with the appearance pinned, and every change ends at 0 frames.

use sill_ui_kit::harness::fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::harness::assert_settles_to_zero_frames;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use ds_settings::Environment;
use sill_model::idle::{DimLevel, DimShown, Idle, IdleMode, IdlePhase};
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::IdleDimOverlay;
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::services::Services;

thread_local! {
    static WORLD: RefCell<Option<(Services, PinnedAppearance)>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let Some((services, appearance)) = WORLD.with(|w| w.borrow().clone()) else {
        return rsx! {};
    };
    use_context_provider(|| services);
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(|| appearance);
    use_context_provider(|| SettingsFeed::fixed(ShellFile::default()));
    rsx! { IdleDimOverlay { output: fake::output() } }
}

const VIEW: Viewport = Viewport {
    width: 320,
    height: 200,
    scale_percent: 100,
};

const SCRIM: &str = ".ds-idle-dim";

/// The feed, and the harness on it.
struct Running {
    harness: Harness,
    feeds: fake::Feeds,
}

fn start(appearance: PinnedAppearance) -> Running {
    let (services, feeds) = fake::services();
    WORLD.with(|w| *w.borrow_mut() = Some((services, appearance)));
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::ZERO);
    assert_eq!(harness.count(SCRIM), 1, "one scrim, mounted at once");
    Running { harness, feeds }
}

fn reduced() -> PinnedAppearance {
    let mut environment = Environment::default();
    environment.settings.appearance.motion_level = ds::Motion::Reduced;
    PinnedAppearance(environment)
}

impl Running {
    /// The idle service publishes `dim`, and the next frame runs.
    fn publish(&mut self, dim: DimShown) {
        let phase = match dim {
            DimShown::Clear => IdlePhase::Active,
            DimShown::Dimmed(_) => IdlePhase::Dimmed,
        };
        self.feeds.idle.send_replace(Idle {
            mode: IdleMode::Own,
            phase,
            dim,
            ..Idle::default()
        });
        self.harness.advance(Duration::ZERO);
        self.harness.advance(Duration::ZERO);
    }

    /// The scrim's opacity in thousandths, from its inline style.
    fn scrim(&self) -> u32 {
        let style = self.harness.attr(SCRIM, "style").unwrap_or_default();
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

    fn advance(&mut self, ms: u64) {
        self.harness.advance(Duration::from_millis(ms));
    }
}

#[test]
fn awake_at_rest_the_scrim_is_clear_and_draws_no_frames() {
    let mut run = start(PinnedAppearance::default());
    assert_eq!(run.scrim(), 0);
    assert_settles_to_zero_frames(&mut run.harness);
    run.publish(DimShown::Clear);
    assert_eq!(run.scrim(), 0, "a Clear while clear changes nothing");
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_dim_fades_in_over_t_idle_dim_to_what_the_display_does_not_keep() {
    let mut run = start(PinnedAppearance::default());
    // `idle.dim_level_pct` 30: the display keeps 30 %, the scrim covers 70 %.
    run.publish(DimShown::Dimmed(DimLevel(30)));
    run.advance(900);
    let mid = run.scrim();
    assert!((1..700).contains(&mid), "mid-fade at 900 ms: {mid}");
    // `--t-idle-dim` is 2000 ms.
    run.advance(1_200);
    assert_eq!(run.scrim(), 700);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_wake_snaps_the_scrim_out_even_mid_fade() {
    let mut run = start(PinnedAppearance::default());
    run.publish(DimShown::Dimmed(DimLevel(50)));
    run.advance(900);
    let mid = run.scrim();
    assert!((1..500).contains(&mid), "mid-fade at 900 ms: {mid}");
    run.publish(DimShown::Clear);
    assert_eq!(run.scrim(), 0, "the next frame is clear");
    assert_settles_to_zero_frames(&mut run.harness);

    // And from a settled dim.
    run.publish(DimShown::Dimmed(DimLevel(50)));
    run.advance(2_500);
    assert_eq!(run.scrim(), 500);
    assert_settles_to_zero_frames(&mut run.harness);
    run.publish(DimShown::Clear);
    assert_eq!(run.scrim(), 0);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_level_edit_while_dimmed_does_not_animate() {
    let mut run = start(PinnedAppearance::default());
    run.publish(DimShown::Dimmed(DimLevel(50)));
    run.advance(2_500);
    assert_eq!(run.scrim(), 500);
    assert_settles_to_zero_frames(&mut run.harness);
    run.publish(DimShown::Dimmed(DimLevel(20)));
    assert_eq!(run.scrim(), 800, "snapped to the new level");
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn reduced_motion_dims_at_once() {
    let mut run = start(reduced());
    run.publish(DimShown::Dimmed(DimLevel(50)));
    assert_eq!(run.scrim(), 500, "no fade at all");
    assert_settles_to_zero_frames(&mut run.harness);
    run.publish(DimShown::Clear);
    assert_eq!(run.scrim(), 0);
    assert_settles_to_zero_frames(&mut run.harness);
}
