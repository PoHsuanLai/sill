//! The hot corner surface in a `ds_harness::Harness` (design/13 §13.3.12): a real pointer
//! dwelling in it fires once, not before the dwell and not twice without a
//! leave and the re-arm. On the virtual clock, so the dwell and the re-arm
//! are checked to the millisecond.

use ds::prelude::*;
use std::time::Duration;

use ds_harness::{Driver, Harness, Input, Viewport};
use ds_settings::Ms;
use shell_host::OutputId;
use sill_overlays::HotCorners;
use sill_settings::{CornerAction, HotCornersSettings, ShellFile};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{Corner, CornerLink};

const DWELL_MS: u16 = 40;
const REARM_MS: u16 = 60;

fn settings() -> HotCornersSettings {
    HotCornersSettings {
        top_left: CornerAction::Launcher,
        dwell_ms: Ms(DWELL_MS),
        rearm_ms: Ms(REARM_MS),
        ..HotCornersSettings::default()
    }
}

/// The top left corner mounted on `hot_corners`, in a small viewport (the real surface is
/// `hot_corners.size_px` square; the test's viewport only needs to be big enough that a point
/// well outside the corner's own `100vw x 100vh` element is still a point the harness can move
/// the pointer to for a leave).
fn start(hot_corners: HotCornersSettings) -> (Harness, CornerLink) {
    let link = CornerLink::new();
    let file = ShellFile {
        hot_corners,
        ..ShellFile::default()
    };
    let running = SurfaceHarness::<HotCorners>::setup(link.clone(), Corner::TopLeft)
        .settings(file)
        .viewport(Viewport {
            width: 4,
            height: 4,
            scale_percent: 100,
        })
        .start();
    (running.harness, link)
}

/// The corner element's own centre (the UA's default margin around `main` means it is not at
/// the viewport's origin).
fn inside(harness: &Harness) -> Point {
    harness
        .centre(".sill-hot-corner")
        .expect("the corner element is laid out")
}

/// Well clear of the corner element, still inside the viewport.
fn outside(harness: &Harness) -> Point {
    let at = inside(harness);
    Point {
        x: at.x + Px(200.0),
        y: at.y + Px(200.0),
    }
}

#[test]
fn a_dwell_fires_once_and_not_before() {
    let (mut harness, link) = start(settings());
    harness.advance(Duration::from_millis(10));
    assert_eq!(link.last(), None);
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(ms(DWELL_MS) - MS);
    assert_eq!(link.last(), None, "not a millisecond before the dwell");
    harness.advance(MS);
    let first = link.last().expect("fired at the dwell");
    assert_eq!(first.corner, Corner::TopLeft);
    assert_eq!(first.output, OutputId(fake::output().id.0));
    // Staying inside fires nothing more.
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert_eq!(
        link.last(),
        Some(first),
        "a pointer parked in the corner acts once"
    );
}

#[test]
fn leaving_before_the_dwell_cancels_it() {
    let (mut harness, link) = start(settings());
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(Duration::from_millis(DWELL_MS as u64 / 2));
    harness.send(Input::pointer_move(outside(&harness)));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    assert_eq!(link.last(), None, "left before the dwell ran out");
}

#[test]
fn leaving_and_the_rearm_lets_it_fire_again() {
    let (mut harness, link) = start(settings());
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 2));
    let first = link.last().expect("the first dwell fired");
    assert_eq!(first.seq, 1);
    // Leaving well before the re-arm is over and coming back starts no new dwell at all (only a
    // leave while re-armed does; entering while `Spent` just notes the pointer is back in), so
    // however long it then stays inside, nothing fires again.
    harness.send(Input::pointer_move(outside(&harness)));
    harness.advance(Duration::from_millis(REARM_MS as u64 / 4));
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 5));
    assert_eq!(
        link.last(),
        Some(first),
        "still within the re-arm, no fresh dwell was armed"
    );
    // Leaving again for past the re-arm, then a fresh dwell fires once more.
    harness.send(Input::pointer_move(outside(&harness)));
    harness.advance(Duration::from_millis(REARM_MS as u64 * 3));
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(Duration::from_millis(DWELL_MS as u64 * 3));
    let second = link.last().expect("re-armed");
    assert_eq!(
        second.seq, 2,
        "a fresh firing after the leave and the re-arm"
    );
}

/// Fire once, leave at once, come back `after` the firing and stay for twice the dwell: the
/// newest firing's sequence number.
fn back_after(after: Duration) -> u64 {
    let (mut harness, link) = start(settings());
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(ms(DWELL_MS));
    assert_eq!(link.last().map(|fired| fired.seq), Some(1));
    harness.send(Input::pointer_move(outside(&harness)));
    harness.advance(after);
    harness.send(Input::pointer_move(inside(&harness)));
    harness.advance(ms(DWELL_MS) * 2);
    link.last().map_or(0, |fired| fired.seq)
}

#[test]
fn the_rearm_ends_exactly_rearm_ms_after_the_firing() {
    assert_eq!(back_after(ms(REARM_MS) - MS), 1, "a millisecond short");
    assert_eq!(back_after(ms(REARM_MS)), 2, "re-armed at the re-arm's end");
}

fn ms(value: u16) -> Duration {
    Duration::from_millis(u64::from(value))
}

const MS: Duration = Duration::from_millis(1);
