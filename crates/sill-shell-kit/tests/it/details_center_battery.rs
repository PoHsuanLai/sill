//! design/26 on a real Blitz document, on the virtual clock: the Battery module's device ring
//! standing at its level, a later level moving it while the percentage changes at once; and the
//! keyboard-brightness level's glyph. Each ends at 0 frames; Reduced shows the level at once.

use dioxus::prelude::*;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::controls::slider_model::SliderLook;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_shell_kit::battery::ring::Readout;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 360,
    height: 200,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

static LEVEL: GlobalSignal<Fraction> = Signal::global(|| Fraction(930));
static MOTION: GlobalSignal<Motion> = Signal::global(|| Motion::Standard);

#[allow(non_snake_case)]
fn Module() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: MOTION(), ..Appearance::default() }, material: Material::Window,
            div { id: "mac", BatteryRing { state: BatteryState { level: LEVEL(), power: BatteryPower::Charging, low_at: LowAt::default() }, label: "This computer", readout: Readout::Under } }
            div { id: "keys", Slider { label: "Keyboard Brightness".to_owned(), value: Fraction(500), glyph: LevelGlyph::KeyboardBrightness, look: SliderLook::Level } }
        }
    }
}

fn figure(harness: &Harness) -> Option<u32> {
    harness
        .text_of("#mac .ds-battery-figure")
        .and_then(|text| text.trim_end_matches('%').parse().ok())
}

/// The level's arc, absent while it is empty.
fn arc(harness: &Harness) -> Option<String> {
    harness.attr("#mac .ds-battery .ds-progress-fill path", "d")
}

#[test]
fn the_ring_stands_at_its_level_and_a_later_level_moves_it_while_the_figure_changes_at_once() {
    let mut harness = virtual_harness(Module);
    assert_eq!(figure(&harness), Some(93), "the figure is the true value");
    let full = arc(&harness);
    assert!(full.is_some(), "the arc is drawn at its level");
    assert_eq!(
        harness
            .attr("#mac .ds-battery .ds-progress", "aria-valuenow")
            .as_deref(),
        Some("93"),
        "the true level is stated from the first frame (R8)"
    );
    assert_eq!(
        harness.count("#mac .ds-battery-bolt"),
        1,
        "the bolt is there"
    );
    assert_settles_to_zero_frames(&mut harness);
    // A later level moves the arc linearly over --t-move; the number does not count.
    harness.within(|| *LEVEL.write() = Fraction(600));
    harness.advance(ms(0));
    assert_eq!(figure(&harness), Some(60));
    harness.advance(ms(100));
    assert_ne!(arc(&harness), full, "the arc is on its way");
    harness.advance(ms(400));
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(figure(&harness), Some(60));
}

#[test]
fn reduced_shows_the_level_at_once() {
    let mut harness = virtual_harness(Module);
    harness.within(|| *MOTION.write() = Motion::Reduced);
    harness.within(|| *LEVEL.write() = Fraction(410));
    harness.advance(ms(0));
    assert_eq!(figure(&harness), Some(41));
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_keyboard_light_draws_a_keyboard_under_a_sun_whose_rays_follow_the_level() {
    let mut harness = virtual_harness(Module);
    for part in ["key-body", "key-core", "key-rays"] {
        assert!(
            harness.count(&format!("#keys [*|data-part={part}]")) >= 1,
            "{part}: {}",
            harness.html()
        );
    }
    assert_eq!(
        harness.count("#keys [*|data-part=rays]"),
        0,
        "not the display's sun"
    );
    assert_settles_to_zero_frames(&mut harness);
}
