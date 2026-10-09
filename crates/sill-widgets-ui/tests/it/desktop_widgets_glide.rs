//! The drag's glide on quire's springs, the real
//! `DesktopWidgets` in a `ds_harness::Harness` on the virtual clock with a pinned appearance:
//! a widget let go while the hand still moves toward its cell carries on past it and
//! springs back (0.8 damping, the release's velocity); let go still, it lands without passing
//! it; under Reduced it never passes it; a widget making room keeps its speed when it is sent
//! back mid-glide, and springs back home. Every moment settles to zero frames.
//!
//! The layer is 1280 x 800: seven 164 px columns from x = 20 (column `c` at 20 + 180 c), rows at
//! 16 + 180 r. The Calendar starts at column 6 row 0 (1100, 16), the Battery under it (1100, 196).

use std::path::PathBuf;
use std::time::Duration;

use ds::prelude::*;
use ds::style::appearance::system::ReducedMotion;
use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{Driver, Harness, Input, Query, Viewport};
use ds_settings::Environment;
use sill_settings::{ShellFile, WidgetKind};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::links::PlacementFile;
use sill_widgets_ui::DesktopWidgets;

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

const FRAME: Duration = Duration::from_millis(16);

/// The desktop's motion preference for a run.
#[derive(Debug, Clone, Copy)]
enum Motion {
    Full,
    Reduced,
}

struct Running {
    harness: Harness,
    _feeds: fake::Feeds,
    dir: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start(name: &str, motion: Motion) -> Running {
    let dir = std::env::temp_dir().join(format!("sill-dw-glide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = PlacementFile(dir.join("desktop-widgets.json"));
    let mut settings = ShellFile::default();
    settings.widgets.desktop_widgets = vec![WidgetKind::Calendar, WidgetKind::Battery];
    let reduce = match motion {
        Motion::Full => ReducedMotion::NoPreference,
        Motion::Reduced => ReducedMotion::Reduce,
    };
    let appearance = PinnedAppearance(Environment {
        system: SystemPrefs::default().with_motion(reduce),
        ..Environment::default()
    });
    let SurfaceHarness {
        mut harness, feeds, ..
    } = SurfaceHarness::<DesktopWidgets>::setup(file, ())
        .settings(settings)
        .viewport(VIEWPORT)
        .appearance(appearance)
        .start();
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    Running {
        harness,
        _feeds: feeds,
        dir,
    }
}

fn point(x: f32, y: f32) -> Point {
    Point { x: Px(x), y: Px(y) }
}

/// The corner of the place holding `id`, in whole px.
fn corner(h: &Harness, id: &str) -> (i32, i32) {
    let rect = h
        .rect(&format!("#{id}"))
        .unwrap_or_else(|| panic!("{id} is laid out"));
    (
        rect.origin.x.0.round() as i32,
        rect.origin.y.0.round() as i32,
    )
}

const CALENDAR: &str = "widget-calendar";
const BATTERY: &str = "widget-battery";

/// The Calendar's centre, 82 px in from its corner.
const GRAB: (f32, f32) = (1182.0, 98.0);

/// Press the Calendar and move the pointer left along its row to `x`, `step` px a frame.
fn sweep_left(h: &mut Harness, x: f32, step: f32) {
    let from = point(GRAB.0, GRAB.1);
    h.send(Input::pointer_move(from));
    h.send(Input::pointer_down(from));
    let mut at = GRAB.0;
    while at > x {
        at = (at - step).max(x);
        h.send(Input::pointer_move(point(at, GRAB.1)));
        h.advance(FRAME);
    }
}

/// The Calendar's corner x on each frame for `frames` frames.
fn trace_x(h: &mut Harness, frames: usize) -> Vec<i32> {
    (0..frames)
        .map(|_| {
            h.advance(FRAME);
            corner(h, CALENDAR).0
        })
        .collect()
}

/// Let the Calendar go with its corner at x = 600 on row 0 (the pointer at 682), after `hold`
/// held still. Its cell is column 3 (560, 16): the release is 40 px short of it, moving toward
/// it at 1500 px/s when not held. Returns its corner x each frame for 1.5 s.
fn throw_short(h: &mut Harness, hold: Duration) -> Vec<i32> {
    sweep_left(h, 682.0, 24.0);
    h.advance(hold);
    h.send(Input::pointer_up(point(682.0, GRAB.1)));
    trace_x(h, 94)
}

/// How far `trace` went past `cell` to the left (0 when it never did).
fn past(trace: &[i32], cell: i32) -> i32 {
    trace.iter().map(|x| cell - x).max().unwrap_or(0).max(0)
}

#[test]
fn a_throw_toward_the_cell_carries_past_it_and_a_still_release_lands() {
    let mut run = start("fast", Motion::Full);
    let thrown = throw_short(&mut run.harness, Duration::ZERO);
    assert_eq!(
        thrown.last(),
        Some(&560),
        "it lands in its cell: {thrown:?}"
    );
    let over = past(&thrown, 560);
    assert!(
        over > 0,
        "a throw toward the cell carries on past it (0.8 damping, the release's speed): \
         {thrown:?}"
    );
    assert!(
        thrown[0] < 600,
        "it keeps going the way the hand went: {thrown:?}"
    );
    assert_settles_to_zero_frames(&mut run.harness);

    let mut run = start("slow", Motion::Full);
    let still = throw_short(&mut run.harness, Duration::from_millis(200));
    assert_eq!(still.last(), Some(&560));
    assert_eq!(
        past(&still, 560),
        0,
        "let go still, it lands critically damped: {still:?}"
    );
    assert!(
        still.windows(2).all(|w| w[1] <= w[0]),
        "straight in: {still:?}"
    );
    let frames_to = |trace: &[i32]| trace.iter().position(|x| *x == 560);
    assert!(
        frames_to(&thrown) < frames_to(&still),
        "the throw reaches the cell sooner: {thrown:?} {still:?}"
    );
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn under_reduced_motion_a_throw_never_passes_its_cell() {
    let mut run = start("reduced", Motion::Reduced);
    let thrown = throw_short(&mut run.harness, Duration::ZERO);
    assert_eq!(thrown.last(), Some(&560), "{thrown:?}");
    assert_eq!(
        past(&thrown, 560),
        0,
        "no thrown velocity and critical damping: {thrown:?}"
    );
    assert_settles_to_zero_frames(&mut run.harness);
}

/// Hold the Calendar over the Battery's cell: the Battery makes room, springing up to (1100, 16).
fn over_the_battery(h: &mut Harness) {
    let from = point(GRAB.0, GRAB.1);
    h.send(Input::pointer_move(from));
    h.send(Input::pointer_down(from));
    for step in 1..=4 {
        h.send(Input::pointer_move(point(
            GRAB.0,
            GRAB.1 + 45.0 * step as f32,
        )));
        h.advance(FRAME);
    }
}

#[test]
fn a_widget_sent_back_mid_glide_keeps_its_speed() {
    let mut run = start("retarget", Motion::Full);
    let h = &mut run.harness;
    over_the_battery(h);
    for _ in 0..5 {
        h.advance(FRAME);
    }
    let (_, going) = corner(h, BATTERY);
    assert!(
        (16..196).contains(&going) && going < 150,
        "on its way up: {going}"
    );
    // Moved on to an empty cell: the Battery's cell is its own again.
    h.send(Input::pointer_move(point(500.0, 600.0)));
    h.advance(FRAME);
    let (_, next) = corner(h, BATTERY);
    assert!(
        next < going,
        "sent back, it still travels up at the speed it had before it turns: {going} then {next}"
    );
    settle_until(h, |h| corner(h, BATTERY) == (1100, 196));
    assert_settles_to_zero_frames(h);
    assert_eq!(corner(h, BATTERY), (1100, 196));
    h.send(Input::pointer_up(point(500.0, 600.0)));
    settle_until(h, |h| corner(h, CALENDAR) == (380, 556));
    assert_settles_to_zero_frames(h);
}

#[test]
fn a_widget_that_made_room_springs_back_home() {
    let mut run = start("back", Motion::Full);
    let h = &mut run.harness;
    over_the_battery(h);
    settle_until(h, |h| corner(h, BATTERY) == (1100, 16));
    assert_settles_to_zero_frames(h);
    h.send(Input::pointer_move(point(500.0, 600.0)));
    let back: Vec<i32> = (0..94)
        .map(|_| {
            h.advance(FRAME);
            corner(h, BATTERY).1
        })
        .collect();
    assert_eq!(back.last(), Some(&196), "home: {back:?}");
    assert!(
        back[0] > 16 && back[0] < 196,
        "it springs, not jumps: {back:?}"
    );
    assert!(
        back.windows(2).all(|w| w[1] >= w[0]),
        "moved aside by another's drag, it goes critically damped, never past home: {back:?}"
    );
    assert_settles_to_zero_frames(h);
    h.send(Input::pointer_up(point(500.0, 600.0)));
    assert_settles_to_zero_frames(h);
}
