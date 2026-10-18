//! design/26 on a real Blitz document, on the virtual clock: a module tile's disc showing the
//! spinner ring while the module is busy, spinning at once and going the moment the module lands
//! on, every moment ending at 0 frames (R3).

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 360,
    height: 120,
    scale_percent: 100,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

/// The module's value and whether it is working towards it.
type Lit = (Check, Availability);

static WIFI: GlobalSignal<Lit> = Signal::global(|| (Check::Off, Availability::Enabled));
static MOTION: GlobalSignal<Motion> = Signal::global(|| Motion::Standard);

fn flip((value, _): Lit) -> Lit {
    (value.flipped(), Availability::Enabled)
}

#[allow(non_snake_case)]
fn Tiles() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { motion: MOTION(), ..Appearance::default() }, material: Material::Window,
            div { style: "display:flex; gap:8px; width:340px",
                div { id: "wifi", style: "flex:1",
                    ModuleTile { glyph: Icon::Wifi, title: "Wi-Fi", status: Some(TextLine::from("On")), value: WIFI().0, availability: WIFI().1, onclick: move |_| *WIFI.write() = flip(WIFI()) }
                }
            }
        }
    }
}

fn set(harness: &mut Harness, state: Lit) {
    harness.within(|| *WIFI.write() = state);
}

fn turn(harness: &Harness) -> Option<String> {
    harness.attr("#wifi .ds-progress", "style")
}

#[test]
fn a_busy_module_spins_its_ring_at_once_and_landing_on_takes_it_away() {
    let mut harness = virtual_harness(Tiles);
    assert_eq!(harness.count("#wifi .ds-progress"), 0);
    assert_settles_to_zero_frames(&mut harness);
    set(&mut harness, (Check::Off, Availability::Busy));
    harness.advance(ms(0));
    assert_eq!(
        harness.attr("#wifi .ds-progress", "data-pending"),
        Some("step".to_owned()),
        "no grace: the ring spins at once"
    );
    let first = turn(&harness);
    harness.advance(ms(83));
    assert_ne!(turn(&harness), first, "a twelfth of a turn a step");
    // Ten seconds on it is still turning: there is no cap.
    harness.advance(ms(10_000));
    assert_eq!(
        harness.attr("#wifi .ds-progress", "data-pending"),
        Some("step".to_owned())
    );
    set(&mut harness, (Check::On, Availability::Enabled));
    harness.advance(ms(0));
    assert_eq!(harness.count("#wifi .ds-progress"), 0);
    assert_settles_to_zero_frames(&mut harness);
}

#[test]
fn reduced_keeps_the_ring_turning() {
    let mut harness = virtual_harness(Tiles);
    harness.within(|| *MOTION.write() = Motion::Reduced);
    harness.advance(ms(20));
    set(&mut harness, (Check::Off, Availability::Busy));
    harness.advance(ms(0));
    let first = turn(&harness);
    harness.advance(ms(83));
    assert_ne!(
        turn(&harness),
        first,
        "the spinner keeps turning under Reduced"
    );
}
