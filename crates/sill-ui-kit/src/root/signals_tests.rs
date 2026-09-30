//! The republished signals reach `Ds`: the seat's modality decides the focus ring's `data-modality`
//! and the surface's scale the root's pixel tokens, each following the seat as it changes. Driven
//! with a stand-in seat because a `SurfaceHandle` cannot be built outside shell-host.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Appearance, Ds, Material, PixelToken};
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use shell_host::{InputModality as SeatModality, Scale as SeatScale};

use super::use_republished;

/// The stand-in seat, in the test's hand.
#[derive(Clone, Copy)]
struct Seat {
    modality: Signal<SeatModality>,
    scale: Signal<SeatScale>,
}

thread_local! {
    static SEAT: RefCell<Option<Seat>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let modality = use_signal(|| SeatModality::Pointer);
    let scale = use_signal(|| SeatScale(120));
    use_hook(|| SEAT.with(|seat| *seat.borrow_mut() = Some(Seat { modality, scale })));
    use_republished(modality.into(), scale.into());
    rsx! {
        Ds { sheet: Some(ds_shell::stylesheet()), appearance: Appearance::default(), material: Material::Window, "content" }
    }
}

fn seat() -> Seat {
    SEAT.with(|seat| *seat.borrow()).expect("the app ran") // test-only
}

fn start() -> Harness {
    let viewport = Viewport {
        width: 200,
        height: 100,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(50));
    harness
}

fn settle(harness: &mut Harness) {
    harness.advance(Duration::from_millis(50));
    harness.advance(Duration::from_millis(50));
}

fn modality(harness: &Harness) -> Option<String> {
    harness.attr(".ds", "data-modality")
}

#[test]
fn the_seats_modality_sets_the_roots_focus_ring_and_follows_it_back() {
    let mut harness = start();
    assert_eq!(modality(&harness).as_deref(), Some("pointer"), "at first");
    seat().modality.set(SeatModality::Keyboard);
    settle(&mut harness);
    assert_eq!(
        modality(&harness).as_deref(),
        Some("keyboard"),
        "after a key"
    );
    seat().modality.set(SeatModality::Pointer);
    settle(&mut harness);
    assert_eq!(
        modality(&harness).as_deref(),
        Some("pointer"),
        "after a press"
    );
}

#[test]
fn the_seats_scale_reaches_the_roots_pixel_tokens() {
    let mut harness = start();
    let style = |harness: &Harness| harness.attr(".ds", "style").unwrap_or_default();
    let scaled = PixelToken::style_attr(ds::Scale(150));
    assert!(!scaled.is_empty(), "1.25x writes tokens");
    assert!(!style(&harness).contains(&scaled), "1x writes none");
    seat().scale.set(SeatScale(150));
    settle(&mut harness);
    assert!(
        style(&harness).contains(&scaled),
        "the new scale is stamped"
    );
}
