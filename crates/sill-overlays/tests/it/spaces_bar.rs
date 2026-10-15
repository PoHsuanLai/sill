//! Mission Control's Spaces bar on the harness, following a `SpacesBarLink` the test writes as
//! the daemon does. Down it draws nothing; up it draws quire's bar with a tile per Space of the
//! output, the one on show marked, a picture on a tile that has one, the tile a drag is over lit,
//! and a plus; a click on a tile or on the plus writes the pick; the bar publishes where its
//! cards lie, and they lie where the bar measures them; and an output Mission Control is not up
//! on gets no bar.

use std::time::Duration;

use ds_harness::{Driver, Harness, Input, Query, Viewport};
use porter_core::SpaceId;
use sill_model::desktop::{
    BarPick, BarShow, DesktopView, Shown, SpaceKind, SpacePicture, SpaceView,
};
use sill_overlays::SpacesBar;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::harness::fake::output_named;
use sill_ui_kit::links::SpacesBarLink;

type Running = SurfaceHarness<SpacesBar>;

/// A small output, tall enough for the menu bar and the expanded bar.
const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 200,
    scale_percent: 100,
};

fn id(text: &str) -> SpaceId {
    SpaceId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn space(name: &str, position: u32, shown: Shown) -> SpaceView {
    SpaceView {
        id: id(name),
        kind: SpaceKind::Desktop,
        output: "DP-1".to_owned(),
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
        .output(output_named(Some("DP-1")))
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

fn tiles(harness: &Harness) -> usize {
    harness.count(".ds-spaces-bar-space")
}

#[test]
fn closed_it_draws_nothing() {
    let running = start();
    assert_eq!(running.harness.count(".ds-spaces-bar"), 0);
    assert_eq!(tiles(&running.harness), 0);
}

#[test]
fn open_it_draws_a_tile_per_space_by_name_with_a_thumbnail_and_a_plus() {
    let running = open();
    assert_eq!(tiles(&running.harness), 2);
    assert_eq!(running.harness.count(".ds-spaces-bar-thumb"), 2);
    let html = running.harness.html();
    assert!(
        html.contains("Desktop 1") && html.contains("Desktop 2"),
        "{html}"
    );
    assert!(html.contains("Add Space"), "{html}");
}

#[test]
fn a_click_on_a_tile_picks_that_space() {
    let mut running = open();
    let first = running
        .harness
        .centre(".ds-spaces-bar-space")
        .expect("a tile is laid out"); // test-only
    running.harness.send(Input::click(first));
    running.harness.advance(Duration::from_millis(100));
    let picked = running.link.last_pick().expect("picked");
    assert_eq!(picked.pick, BarPick::Switch(id("ws-1")));
}

#[test]
fn a_click_on_the_plus_adds_a_space_on_this_output() {
    let mut running = open();
    let plus = running
        .harness
        .centre(".ds-button")
        .expect("the plus is laid out"); // test-only
    running.harness.send(Input::click(plus));
    running.harness.advance(Duration::from_millis(100));
    let picked = running.link.last_pick().expect("picked");
    assert_eq!(picked.pick, BarPick::Add("DP-1".to_owned()));
}

#[test]
fn lowering_the_bar_lifts_it_away_and_withdraws_its_cards() {
    let mut running = open();
    assert_eq!(running.link.latest().layouts.0.len(), 1);
    running.link.set_show(BarShow::Hide);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(running.harness.count(".ds-spaces-bar"), 0);
    assert!(running.link.latest().layouts.0.is_empty());
}

#[test]
fn an_output_mission_control_is_not_up_on_gets_no_bar() {
    let mut running = start();
    running.link.set_view(&two_spaces());
    running.link.set_output("DP-1", Shown::Hidden);
    running.link.set_output("DP-2", Shown::Shown);
    running.link.set_show(BarShow::Show);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(running.harness.count(".ds-spaces-bar"), 0);
    running.link.set_output("DP-1", Shown::Shown);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(running.harness.count(".ds-spaces-bar"), 1);
}

#[test]
fn a_picture_that_arrived_is_on_its_tile_and_a_drag_over_a_tile_lights_it() {
    let mut running = open();
    assert_eq!(running.harness.count(".ds-spaces-bar-picture"), 0);
    running
        .link
        .set_picture("DP-1", id("ws-2"), &SpacePicture::Png(vec![0x89, b'P']));
    running.link.set_drop("DP-1", Some(id("ws-1")));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-spaces-bar-picture"), 1);
    let html = running.harness.html();
    assert!(html.contains("data:image/png;base64,"), "{html}");
    assert_eq!(html.matches("data-drop=\"target\"").count(), 1, "{html}");
    running.link.set_drop("DP-1", None);
    running.harness.advance(Duration::from_millis(100));
    assert!(!running.harness.html().contains("data-drop=\"target\""));
}

#[test]
fn the_published_cards_lie_where_the_bar_draws_its_thumbnails() {
    let running = open();
    let layout = running
        .link
        .latest()
        .layouts
        .0
        .get("DP-1")
        .cloned()
        .expect("the bar published its layout"); // test-only
    assert_eq!(layout.thumb, (160, 100));
    assert_eq!(layout.tiles.len(), 2);
    assert_eq!(layout.tiles[0].space, id("ws-1"));
    let drawn = running
        .harness
        .rect(".ds-spaces-bar-thumb")
        .expect("a thumbnail is laid out"); // test-only
    let (x, y) = (
        drawn.origin.x.0.round() as i32,
        drawn.origin.y.0.round() as i32,
    );
    assert_eq!(layout.tiles[0].rect, (x, y, 160, 100), "{drawn:?}");
}
