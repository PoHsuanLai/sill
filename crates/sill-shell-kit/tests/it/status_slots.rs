//! On a real Blitz document, on the virtual clock: a status glyph in the bar's
//! status item (same box, ink, pill and label as an icon, sized by the setting), the control
//! center's panel header and tile disc holding a status glyph, and the Sound module's level
//! reading the bar's `VolumeState`. Each motion ends at 0 frames.

use dioxus::prelude::*;
use ds::base::vocab::Muting;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::content::status::volume::{VolumeState, VolumeWaves};
use ds::components::content::status::wifi_state::{WifiBars, WifiReach, WifiState};
use ds::components::controls::button_model::ImagePosition;
use ds::components::controls::slider_model::SliderLook;
use ds::motion::detail::stamp::EventStamp;
use ds::prelude::*;
use ds::style::tokens::status::StatusMetrics;
use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 320,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

fn battery(level: u16, power: BatteryPower) -> BatteryState {
    BatteryState {
        level: Fraction(level),
        power,
        low_at: LowAt(Fraction(200)),
    }
}

/// The bar's metrics under test: a 26 px box holding an 18 px glyph, so a glyph drawn at its own
/// written size (16) or at the box would both be caught.
const METRICS: StatusMetrics = StatusMetrics {
    box_size: Px(26.0),
    glyph: Px(18.0),
};

// ---- a status glyph in the bar's status item -------------------------------------------

static ITEM: GlobalSignal<StatusState> = Signal::global(|| {
    StatusState::Wifi(WifiState::Joined {
        bars: WifiBars::Two,
        reach: WifiReach::Internet,
    })
});

#[allow(non_snake_case)]
fn Bar() -> Element {
    let status = ITEM();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { id: "bar", style: METRICS.style_attr(),
                MenuBarItem {
                    image: ImagePosition::Only,
                    icon: status,
                    label: status.words(),
                    onclick: |_| {},
                }
            }
        }
    }
}

const ITEM_SEL: &str = "#bar .ds-menu-bar-item";

fn side(harness: &Harness, selector: &str) -> (f32, f32) {
    let rect = harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is missing"));
    (rect.size.width.0, rect.size.height.0)
}

#[test]
fn a_status_glyph_fills_the_status_items_glyph_square_with_its_label() {
    let mut harness = virtual_harness(Bar);
    let glyph = format!("{ITEM_SEL} > .ds-menu-bar-item-icon > .ds-status-glyph[*|data-kind=wifi]");
    assert_eq!(harness.count(&glyph), 1, "{}", harness.html());
    assert_eq!(
        side(&harness, ITEM_SEL),
        (30.0, 26.0),
        "the item's box: the bar's 30 wide slot"
    );
    assert_eq!(side(&harness, &glyph), (18.0, 18.0), "the setting's glyph");
    let part = format!("{glyph} > .ds-status-part[*|data-part=arc-3]");
    assert_eq!(side(&harness, &part), (18.0, 18.0), "each part fills it");
    let words = harness.within(|| ITEM.peek().words());
    assert_eq!(harness.attr(ITEM_SEL, "aria-label"), Some(words));
    assert_eq!(
        harness.attr(ITEM_SEL, "data-image").as_deref(),
        Some("only")
    );

    // The volume glyph sizes itself inline; the item's square still wins.
    harness.within(|| *ITEM.write() = StatusState::Volume(VolumeState::Heard(VolumeWaves::Two)));
    harness.advance(ms(0));
    let volume =
        format!("{ITEM_SEL} > .ds-menu-bar-item-icon > .ds-status-glyph[*|data-kind=volume]");
    assert_eq!(side(&harness, &volume), (18.0, 18.0), "{}", harness.html());
    let wave = format!("{volume} .ds-level-part[*|data-part=wave-1]");
    assert_eq!(side(&harness, &wave), (18.0, 18.0));
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn the_status_item_plays_the_glyphs_own_moments() {
    let mut harness = virtual_harness(Bar);
    harness.within(|| *ITEM.write() = StatusState::Wifi(WifiState::Idle));
    harness.advance(ms(0));
    let arc = |h: &Harness| {
        h.attr(
            &format!("{ITEM_SEL} .ds-status-part[*|data-part=arc-3]"),
            "data-show",
        )
    };
    assert_eq!(arc(&harness).as_deref(), Some("faint"));
    assert_settles_to_zero_frames(&mut harness);
    // Joining runs the searching loop at once, one layer lit at a time.
    harness.within(|| *ITEM.write() = StatusState::Wifi(WifiState::Joining(EventStamp(1))));
    harness.advance(ms(1_000));
    let lit = harness.count(&format!("{ITEM_SEL} .ds-status-part[*|data-show=lit]"));
    assert_eq!(lit, 1, "one layer lit while searching: {}", harness.html());
    harness.within(|| {
        *ITEM.write() = StatusState::Wifi(WifiState::Joined {
            bars: WifiBars::Three,
            reach: WifiReach::Internet,
        })
    });
    settle_until(&mut harness, |h| arc(h).as_deref() == Some("lit"));
    assert_settles_to_zero_frames(&mut harness);
}

// ---- the control center's slots ------------------------------------------------------

#[allow(non_snake_case)]
fn Center() -> Element {
    let status = StatusState::Battery(battery(930, BatteryPower::Battery));
    let wifi = StatusState::Wifi(WifiState::Joined {
        bars: WifiBars::Three,
        reach: WifiReach::NoInternet,
    });
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { id: "panel",
                ModulePanel { glyph: IconSource::Status(status), title: "Battery",
                    p { "93 %" }
                }
            }
            div { id: "tile",
                ModuleTile {
                    glyph: wifi,
                    title: "Wi-Fi",
                    status: Some(wifi.words().into()),
                    value: Check::On,
                    onclick: |_| {},
                }
            }
            div { id: "plain",
                ModuleTile { glyph: Icon::Moon, title: "Focus", status: None, value: Check::Off, onclick: |_| {} }
            }
        }
    }
}

/// The battery fill's drawn width on the 24 grid, 0 when there is none.
fn fill(harness: &Harness) -> f32 {
    harness
        .attr("#panel [*|data-part=fill] rect", "width")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
}

#[test]
fn the_panel_header_holds_the_battery_and_the_disc_holds_the_wifi_glyph() {
    let mut harness = virtual_harness(Center);
    assert_eq!(
        harness.count("#panel .ds-module-panel-glyph > .ds-status-glyph[*|data-kind=battery]"),
        1,
        "{}",
        harness.html()
    );
    // The fill stands at its level on first show: nothing sweeps in (91 % of the solid fill box, 14.5 units).
    assert!(
        (fill(&harness) - 13.18).abs() < 0.1,
        "full: {}",
        fill(&harness)
    );
    let disc = "#tile .ds-module-disc > .ds-status-glyph[*|data-kind=wifi]";
    assert_eq!(harness.count(disc), 1, "{}", harness.html());
    assert_eq!(
        harness.attr(
            &format!("{disc} > .ds-status-part[*|data-part=badge]"),
            "data-show"
        ),
        Some("lit".to_owned()),
        "the no-internet badge"
    );
    assert_eq!(side(&harness, disc), (16.0, 16.0), "the disc's glyph size");
    assert_eq!(
        harness.count("#plain .ds-module-disc > svg.ds-ic"),
        1,
        "an Icon still draws"
    );
    assert_settles_to_zero_frames(&mut harness);
}

// ---- the Sound module reads the bar's VolumeState ------------------------------------

static VOLUME: GlobalSignal<VolumeState> = Signal::global(|| VolumeState::Muted);

#[allow(non_snake_case)]
fn Sound() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { id: "shared", style: "width: 300px",
                Slider { label: "Volume", value: Fraction(500), glyph: VOLUME(), onchange: |_| {}, look: SliderLook::Capsule }
            }
            div { id: "own", style: "width: 300px",
                Slider { label: "Output", value: Fraction(500), glyph: LevelGlyph::Volume(Muting::Audible), onchange: |_| {}, look: SliderLook::Capsule }
            }
        }
    }
}

fn part_on(harness: &Harness, root: &str, part: &str) -> Option<String> {
    harness.attr(
        &format!("{root} .ds-slider-icon .ds-level-part[*|data-part={part}]"),
        "data-on",
    )
}

#[test]
fn a_level_given_a_volume_state_draws_that_states_waves_and_slash() {
    let mut harness = virtual_harness(Sound);
    // Muted at a level of 50 %: the slash, no waves, as the bar's glyph draws it.
    assert_eq!(part_on(&harness, "#shared", "slash").as_deref(), Some("on"));
    assert_eq!(
        part_on(&harness, "#shared", "wave-1").as_deref(),
        Some("off")
    );
    // A LevelGlyph still follows the value: two waves at 50 %.
    assert_eq!(part_on(&harness, "#own", "wave-2").as_deref(), Some("on"));
    assert_eq!(part_on(&harness, "#own", "slash").as_deref(), Some("off"));
    harness.within(|| *VOLUME.write() = VolumeState::Heard(VolumeWaves::Three));
    harness.advance(ms(0));
    assert_eq!(
        part_on(&harness, "#shared", "slash").as_deref(),
        Some("off")
    );
    assert_eq!(
        part_on(&harness, "#shared", "wave-3").as_deref(),
        Some("on"),
        "the state's three waves, whatever the capsule's 50 %"
    );
    assert_eq!(
        harness
            .attr("#shared .ds-slider", "aria-valuenow")
            .as_deref(),
        Some("50"),
        "the capsule still shows the value"
    );
    assert_settles_to_zero_frames(&mut harness);
}
