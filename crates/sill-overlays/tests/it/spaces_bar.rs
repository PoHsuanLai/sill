//! Mission Control's Spaces bar on the harness, following a `SpacesBarLink` the test writes (and
//! the desktop it follows) as the daemon does. Down it draws nothing; up it draws quire's bar with a tile per Space of the
//! output, the one on show marked, a picture on every tile that has one (a hidden Space's too), the
//! output's wallpaper under the pictures, the tile a drag is over lit, and a plus that lights when
//! the drag is over it; a click on a tile or on the plus writes the pick; the bar publishes where
//! its cards and its plus lie, and they lie where the bar draws them; and an output Mission
//! Control is not up on gets no bar.

use std::time::Duration;

use ds_harness::{Harness, Input, Viewport};
use porter_core::SpaceId;
use sill_model::desktop::{
    BarPick, BarShow, DesktopView, Shown, SpaceKind, SpacePicture, SpaceView,
};
use sill_model::ids::OutputName;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};
use sill_overlays::SpacesBar;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::harness::fake::output_named;
use sill_ui_kit::links::SpacesBarLink;
use tokio::sync::watch;

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

/// The bar with Mission Control closed, following `desktop`.
fn start_on(desktop: watch::Receiver<DesktopView>) -> Running {
    let mut running = SurfaceHarness::<SpacesBar>::setup(SpacesBarLink::following(desktop), ())
        .output(output_named(Some("DP-1")))
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running
}

/// The bar with Mission Control closed, over a desktop that is `view` and stays so.
fn start(view: DesktopView) -> Running {
    start_on(watch::channel(view).1)
}

/// The bar with Mission Control open on two Spaces.
fn open() -> Running {
    let mut running = start(two_spaces());
    running.link.set_show(BarShow::Show);
    running.harness.advance(Duration::from_millis(400));
    running
}

fn tiles(harness: &Harness) -> usize {
    harness.count(".ds-spaces-bar-space")
}

#[test]
fn closed_it_draws_nothing() {
    let running = start(two_spaces());
    assert_eq!(running.harness.count(".ds-spaces-bar"), 0);
    assert_eq!(tiles(&running.harness), 0);
}

#[test]
fn a_space_the_desktop_gains_is_a_tile_at_once() {
    let (desktop, view) = watch::channel(two_spaces());
    let mut running = start_on(view);
    running.link.set_show(BarShow::Show);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(tiles(&running.harness), 2);
    let mut three = two_spaces();
    three.spaces.push(space("ws-3", 2, Shown::Hidden));
    desktop.send_replace(three);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(tiles(&running.harness), 3);
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
    assert_eq!(picked.item, BarPick::Switch(id("ws-1")));
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
    assert_eq!(picked.item, BarPick::Add("DP-1".to_owned()));
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
    let mut running = start(two_spaces());
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

/// `selector`'s element as drawn, in whole logical pixels.
fn drawn(running: &Running, selector: &str) -> (i32, i32, u32, u32) {
    let rect = running
        .harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out")); // test-only
    (
        rect.origin.x.0.round() as i32,
        rect.origin.y.0.round() as i32,
        rect.size.width.0.round() as u32,
        rect.size.height.0.round() as u32,
    )
}

fn published(running: &Running) -> sill_model::desktop::BarLayout {
    running
        .link
        .latest()
        .layouts
        .0
        .get("DP-1")
        .cloned()
        .expect("the bar published its layout") // test-only
}

#[test]
fn the_published_cards_lie_where_the_bar_draws_its_thumbnails() {
    let running = open();
    let layout = published(&running);
    assert_eq!(layout.thumb, (160, 100));
    assert_eq!(layout.tiles.len(), 2);
    assert_eq!(layout.tiles[0].space, id("ws-1"));
    assert_eq!(
        layout.tiles[0].rect,
        drawn(&running, ".ds-spaces-bar-thumb")
    );
}

#[test]
fn the_published_plus_lies_where_the_bar_draws_it() {
    let running = open();
    assert_eq!(
        published(&running).add,
        Some(drawn(&running, ".ds-spaces-bar-add"))
    );
}

#[test]
fn every_space_shows_its_picture_the_hidden_one_too() {
    let mut running = open();
    for space in ["ws-1", "ws-2"] {
        running
            .link
            .set_picture("DP-1", id(space), &SpacePicture::Png(vec![0x89, b'P']));
    }
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-spaces-bar-picture"), 2);
}

#[test]
fn the_plus_lights_while_the_dragged_window_is_over_it_and_not_with_a_tile() {
    let mut running = open();
    let lit = ".ds-spaces-bar-add[data-drop=target]";
    assert_eq!(running.harness.count(lit), 0);
    running.link.set_drop_new("DP-1", Shown::Shown);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(lit), 1);
    // Only the plus is lit, no tile.
    assert_eq!(running.harness.count("[data-drop=target]"), 1);
    running.link.set_drop_new("DP-1", Shown::Hidden);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(lit), 0);
}

#[test]
fn the_outputs_wallpaper_lies_under_the_pictures_and_another_outputs_does_not() {
    let mut running = open();
    let backdrops = |running: &Running| running.harness.count("img.ds-spaces-bar-backdrop");
    assert_eq!(backdrops(&running), 0);
    let image = |output: &str| Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName(output.to_owned()),
            light: "/tmp/sill-bar-wallpaper/light.png".into(),
            dark: "/tmp/sill-bar-wallpaper/dark.png".into(),
        }],
    };
    running.feeds.wallpaper.send_replace(image("DP-2"));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(backdrops(&running), 0);
    running.feeds.wallpaper.send_replace(image("DP-1"));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(backdrops(&running), 2);
    let html = running.harness.html();
    assert!(html.contains("/tmp/sill-bar-wallpaper/light.png"), "{html}");
}
