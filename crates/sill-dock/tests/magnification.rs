//! The dock's hover label, as design/10 §10.3.4 draws it: the name stands 7 px above the plate,
//! centred on the tile (quire's own card lay in the pill-sized root's coordinates and landed the
//! pill's origin away from its tile, then overlapped the tile). The magnified geometry, the plate
//! widening and the baseline, is `dock::frame`'s tests (a harness has no layer surface to drive
//! the frames that grow a tile).

use std::time::Duration;

use ds::prelude::{Point, Px};
use ds_harness::{Driver, Input, Query};
use sill_dock::Dock;
use sill_ui_kit::harness::{SurfaceHarness, fake};

/// What a rect is: its left, top, right and bottom edges.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Edges {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

fn edges(run: &SurfaceHarness<Dock>, selector: &str) -> Edges {
    let rect = run
        .harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is drawn"));
    Edges {
        left: rect.origin.x.0,
        top: rect.origin.y.0,
        right: rect.origin.x.0 + rect.size.width.0,
        bottom: rect.origin.y.0 + rect.size.height.0,
    }
}

fn tile(name: &str) -> String {
    format!(".ds-dock-tile[*|aria-label='{name}']")
}

fn dock() -> SurfaceHarness<Dock> {
    let mut run = SurfaceHarness::<Dock>::setup((), ()).start();
    run.feeds.apps.send_replace(fake::dock::index());
    run.feeds.pins.send_replace(fake::dock::pins());
    run.feeds.badges.send_replace(fake::dock::badges());
    run.feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = fake::dock::toplevels_on("DP-1"));
    run.harness.advance(Duration::from_millis(40));
    run
}

/// The pointer on the Trash tile's middle, then nudged a pixel at a time while time passes: with no
/// layer surface to drive frames in a harness, each move draws one at the clock's time, and the
/// enter ramp (120 ms) and the label's delay (100 ms) are over by the last.
fn hover_trash(run: &mut SurfaceHarness<Dock>) {
    let at = run
        .harness
        .centre(&tile("Trash"))
        .expect("the Trash tile is drawn");
    for step in 0..6 {
        let nudge = if step % 2 == 0 { 0.0 } else { 1.0 };
        run.harness.send(Input::pointer_move(Point {
            x: Px(at.x.0 + nudge),
            y: at.y,
        }));
        run.harness.advance(Duration::from_millis(100));
    }
}

#[test]
fn the_name_stands_above_the_magnified_tile_centred_on_it() {
    let mut run = dock();
    hover_trash(&mut run);
    let label = edges(&run, ".sill-dock-label-text");
    let plate = edges(&run, &format!("{} .ds-dock-plate", tile("Trash")));
    let tile = edges(&run, &tile("Trash"));
    assert!(
        (plate.top - label.bottom - 7.0).abs() <= 1.0,
        "7 px above the plate: label {label:?}, plate {plate:?}"
    );
    assert!(
        label.bottom <= tile.top + (plate.top - tile.top),
        "clear of the tile"
    );
    let (label_centre, tile_centre) = (
        (label.left + label.right) / 2.0,
        (tile.left + tile.right) / 2.0,
    );
    assert!(
        (label_centre - tile_centre).abs() <= 1.0,
        "centred: {label_centre} vs {tile_centre}"
    );
}

#[test]
fn no_name_is_up_until_a_tile_is_hovered() {
    let run = dock();
    assert_eq!(run.harness.count(".sill-dock-label"), 0);
}
