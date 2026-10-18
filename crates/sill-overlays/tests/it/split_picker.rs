//! The Split View picker on the harness, following a `SplitPickerLink` the test writes as the
//! daemon does. Hidden it draws nothing; an offer draws the dimmed backdrop and, in the half the
//! first window left free, one tile per window on offer; a click on a tile picks it, Escape or a
//! press anywhere else turns the choice down, and either hides the picker at once.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use sill_model::desktop::{SplitCard, SplitSide};
use sill_overlays::SplitPicker;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::{SplitAnswer, SplitPickerLink};

type Running = SurfaceHarness<SplitPicker>;

/// A small output.
const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn card(window: &str) -> SplitCard {
    SplitCard {
        window: window.to_owned(),
        app: "term".to_owned(),
        title: format!("Title of {window}"),
    }
}

/// The picker with nothing on offer.
fn start() -> Running {
    let mut running = SurfaceHarness::<SplitPicker>::setup(SplitPickerLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running
}

/// The picker with `w1` full screen on the left and `w2`, `w3` on offer.
fn offered() -> Running {
    let mut running = start();
    running.link.begin("w1", SplitSide::Left, "");
    running.link.add(card("w2"));
    running.link.add(card("w3"));
    running.harness.advance(Duration::from_millis(200));
    running
}

fn tiles(harness: &Harness) -> usize {
    harness.count(".ds-switcher-cell")
}

#[test]
fn hidden_it_draws_nothing() {
    let running = start();
    assert_eq!(running.harness.count(".sill-split"), 0);
    assert_eq!(tiles(&running.harness), 0);
}

#[test]
fn an_offer_draws_the_backdrop_and_a_tile_per_window_in_the_free_half() {
    let running = offered();
    assert_eq!(running.harness.count(".sill-split-scrim"), 1);
    assert_eq!(tiles(&running.harness), 2, "w2 and w3");
    assert!(
        running.harness.html().contains("data-half=\"right\""),
        "the first window took the left, so the tiles fill the right"
    );
}

#[test]
fn escape_turns_the_choice_down_and_hides_the_picker() {
    let mut running = offered();
    assert_eq!(running.link.last_answer(), None);
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(100));
    let answered = running.link.last_answer().expect("answered");
    assert_eq!(
        (answered.first.as_str(), answered.answer),
        ("w1", SplitAnswer::Cancelled)
    );
    assert_eq!(running.harness.count(".sill-split"), 0);
}

#[test]
fn a_press_on_the_first_windows_half_turns_the_choice_down() {
    let mut running = offered();
    running.harness.send(Input::click(Point {
        x: Px(20.0),
        y: Px(780.0),
    }));
    running.harness.advance(Duration::from_millis(100));
    let answered = running.link.last_answer().expect("answered");
    assert_eq!(answered.answer, SplitAnswer::Cancelled);
    assert_eq!(running.harness.count(".sill-split"), 0);
}

#[test]
fn a_click_on_a_tile_picks_that_window() {
    let mut running = offered();
    let first = running
        .harness
        .centre(".ds-switcher-cell")
        .expect("a tile is laid out"); // test-only
    running.harness.send(Input::click(first));
    running.harness.advance(Duration::from_millis(100));
    let answered = running.link.last_answer().expect("answered");
    assert_eq!(
        (answered.first.as_str(), answered.answer),
        ("w1", SplitAnswer::Picked("w2".to_owned())),
        "the first tile is the most recently used window"
    );
    assert_eq!(running.harness.count(".sill-split"), 0);
}

#[test]
fn the_compositor_ending_the_offer_takes_the_picker_away() {
    let mut running = offered();
    running.link.end("w1");
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".sill-split"), 0);
    assert_eq!(running.link.last_answer(), None, "nobody answered");
}
