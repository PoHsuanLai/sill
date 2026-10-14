//! A dock tile on a real Blitz document: its plate fills the tile, its
//! label hangs on the plate's own box (the label's anchor is the plate, not a zero-size box at the
//! tile's corner), and a bounce's lift raises the plate while the running dot stays on the
//! baseline.

use crate::support::probe;

use dioxus::prelude::*;
use ds::base::vocab::Activity;
use ds::prelude::*;
use ds::style::icon::family::PlateFamily;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Viewport};
use probe::rect;
use sill_shell_kit::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 320,
    height: 240,
    scale_percent: 100,
};

/// The bounce's lift in whole pixels: the document may be built on another thread.
static LIFT: AtomicU32 = AtomicU32::new(0);

#[allow(non_snake_case)]
fn Tile() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "position:absolute; left:100px; top:100px",
                DockTile {
                    icon: IconSource::Glyph(Icon::Folder),
                    plate: Some(PlateFamily::Blue),
                    running: Activity::Active,
                    side: Px(48.0),
                    lift: Px(LIFT.load(Ordering::Relaxed) as f32),
                    label: "Files",
                    label_shown: Some(Shown::Visible),
                    onclick: |_| {},
                }
            }
        }
    }
}

fn laid_out(lift: u32) -> Harness {
    LIFT.store(lift, Ordering::Relaxed);
    let mut harness = Harness::new(Tile, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(400));
    harness
}

#[test]
fn the_label_hangs_on_the_plate_and_the_bounce_lifts_it_but_not_the_dot() {
    let rest = laid_out(0);
    let plate = rect(&rest, ".ds-dock-plate");
    let target = rect(&rest, ".ds-hover-target");
    assert_eq!(
        (plate.size.width.0, plate.size.height.0),
        (48.0, 48.0),
        "the plate fills a 48 px tile"
    );
    assert!(
        (target.size.width.0 - plate.size.width.0).abs() < 1.0
            && (target.size.height.0 - plate.size.height.0).abs() < 1.0,
        "the label's anchor is the plate's box: {target:?} {plate:?}"
    );
    let label = rect(&rest, ".ds-dock-label");
    assert!(
        label.origin.y.0 + label.size.height.0 <= plate.origin.y.0,
        "the label stands above the plate: {label:?} {plate:?}"
    );
    let dot = rect(&rest, ".ds-running-dot");

    let lifted = laid_out(10);
    let raised = rect(&lifted, ".ds-dock-plate");
    assert!(
        (plate.origin.y.0 - raised.origin.y.0 - 10.0).abs() < 0.5,
        "the plate rises by the lift: {plate:?} {raised:?}"
    );
    assert_eq!(
        rect(&lifted, ".ds-running-dot"),
        dot,
        "the dot stays on the baseline"
    );
}
