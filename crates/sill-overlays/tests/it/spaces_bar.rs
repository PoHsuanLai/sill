//! Mission Control's Spaces bar on the harness, following a `SpacesBarLink` the test writes as
//! the daemon does. Down it draws nothing; up it draws a pill per Space of the output with the
//! one on show marked and a plus; a click on a pill or on the plus writes the pick; lowering the
//! bar plays the leave and then takes the strip away.

use std::time::Duration;

use ds_harness::{Driver, Harness, Input, Query, Viewport};
use porter_core::SpaceId;
use sill_model::desktop::{BarPick, BarShow, DesktopView, Shown, SpaceKind, SpaceView};
use sill_overlays::SpacesBar;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::SpacesBarLink;

type Running = SurfaceHarness<SpacesBar>;

/// A small output.
const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 112,
    scale_percent: 100,
};

fn id(text: &str) -> SpaceId {
    SpaceId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn space(name: &str, position: u32, shown: Shown) -> SpaceView {
    SpaceView {
        id: id(name),
        kind: SpaceKind::Desktop,
        output: String::new(),
        position,
        shown,
    }
}

/// The desktop with two Spaces, the second on show.
fn two_spaces() -> DesktopView {
    DesktopView {
        spaces: vec![
            space("ws-1", 0, Shown::Hidden),
            space("ws-2", 1, Shown::Shown),
        ],
        ..DesktopView::default()
    }
}

/// The bar with Mission Control closed.
fn start() -> Running {
    let mut running = SurfaceHarness::<SpacesBar>::setup(SpacesBarLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running
}

/// The bar with Mission Control open on two Spaces.
fn open() -> Running {
    let mut running = start();
    running.link.set_view(&two_spaces());
    running.link.set_show(BarShow::Show);
    running.harness.advance(Duration::from_millis(400));
    running
}

fn pills(harness: &Harness) -> usize {
    harness.count(".ds-ws-pill")
}

#[test]
fn closed_it_draws_nothing() {
    let running = start();
    assert_eq!(running.harness.count(".sill-spaces-bar"), 0);
    assert_eq!(pills(&running.harness), 0);
}

#[test]
fn open_it_draws_a_pill_per_space_by_name_and_a_plus() {
    let running = open();
    assert_eq!(pills(&running.harness), 2);
    let html = running.harness.html();
    assert!(
        html.contains("Desktop 1") && html.contains("Desktop 2"),
        "{html}"
    );
    assert!(html.contains("Add Space"), "{html}");
}

#[test]
fn a_click_on_a_pill_picks_that_space() {
    let mut running = open();
    let first = running
        .harness
        .centre(".ds-ws-pill")
        .expect("a pill is laid out"); // test-only
    running.harness.send(Input::click(first));
    running.harness.advance(Duration::from_millis(100));
    let picked = running.link.last_pick().expect("picked");
    assert_eq!(picked.pick, BarPick::Switch(id("ws-1")));
}

#[test]
fn a_click_on_the_plus_adds_a_space() {
    let mut running = open();
    let plus = running
        .harness
        .centre(".ds-button")
        .expect("the plus is laid out"); // test-only
    running.harness.send(Input::click(plus));
    running.harness.advance(Duration::from_millis(100));
    let picked = running.link.last_pick().expect("picked");
    assert!(matches!(picked.pick, BarPick::Add(_)), "{:?}", picked.pick);
}

#[test]
fn lowering_the_bar_leaves_and_then_takes_the_strip_away() {
    let mut running = open();
    running.link.set_show(BarShow::Hide);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(running.harness.count(".sill-spaces-bar"), 0);
}
