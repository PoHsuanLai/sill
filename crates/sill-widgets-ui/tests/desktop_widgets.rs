//! What the desktop widget layer draws, the real `DesktopWidgets` mounted as a `Surface` on the
//! fake services: `widgets.desktop_widgets` placed on the grid from the top right, each in its
//! Widget-material frame at its cell's box; a saved placement kept; the placements written back
//! to the file the surface was given (a scratch file here: never the user's state); and nothing
//! painted between the widgets.

use std::path::PathBuf;
use std::time::Duration;

use ds_harness::{Harness, Viewport};
use sill_settings::{ShellFile, WidgetKind};
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::PlacementFile;
use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
use sill_widgets_ui::DesktopWidgets;

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running layer.
struct Running {
    harness: Harness,
    settings: tokio::sync::watch::Sender<ShellFile>,
    _scratch: Scratch,
}

/// A scratch directory, removed when the test ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The scratch directory and places file of the test named `name`.
fn temp_file(name: &str) -> (PathBuf, PlacementFile) {
    let dir = std::env::temp_dir().join(format!("sill-dw-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = PlacementFile(dir.join("desktop-widgets.json"));
    (dir, file)
}

/// The layer on `file`'s settings with nothing saved.
fn start(name: &str, file: ShellFile) -> Running {
    let (dir, places) = temp_file(name);
    start_in(dir, places, file)
}

/// The layer on `file`'s settings and the places file `places`, settled.
fn start_in(dir: PathBuf, places: PlacementFile, file: ShellFile) -> Running {
    let run = SurfaceHarness::<DesktopWidgets>::setup(places, ())
        .settings(file)
        .viewport(VIEWPORT)
        .start();
    let SurfaceHarness {
        mut harness,
        settings,
        ..
    } = run;
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    Running {
        harness,
        settings,
        _scratch: Scratch(dir),
    }
}

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

/// `selector`'s box as whole px: (left, top, width, height).
fn box_of(h: &Harness, selector: &str) -> (i32, i32, i32, i32) {
    let rect = h
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out"));
    let px = |v: f32| v.round() as i32;
    (
        px(rect.origin.x.0),
        px(rect.origin.y.0),
        px(rect.size.width.0),
        px(rect.size.height.0),
    )
}

#[test]
fn the_default_widgets_fill_from_the_top_right_in_widget_frames() {
    // 1280 px: seven 164 px cells and eight 16 px gaps take 1276, so the grid starts at 4.
    let Running { harness: h, .. } = start("default", ShellFile::default());
    assert_eq!(h.count(".ds-widget"), 2);
    assert_eq!(box_of(&h, "#widget-calendar"), (1100, 16, 164, 164));
    assert_eq!(box_of(&h, "#widget-battery"), (1100, 196, 164, 164));
    for id in ["#widget-calendar", "#widget-battery"] {
        assert_eq!(
            h.count(&format!(".ds[*|data-material=widget] > .ds-widget{id}")),
            1,
            "{id}"
        );
    }
    // The grid wrapper carries the settings' cell and gap as quire's widget tokens.
    assert_eq!(
        h.attr(".sill-dw-grid", "style").as_deref(),
        Some("--widget-cell-px:164px;--widget-gap-px:16px;")
    );
}

#[test]
fn a_medium_widget_takes_two_cells() {
    let mut file = ShellFile::default();
    file.widgets.desktop_widgets = vec![WidgetKind::NowPlaying, WidgetKind::WorldClock];
    let Running { harness: h, .. } = start("medium", file);
    assert_eq!(box_of(&h, "#widget-now-playing"), (920, 16, 344, 164));
    assert_eq!(box_of(&h, "#widget-world-clock"), (920, 196, 344, 164));
}

#[test]
fn saved_places_are_kept_and_the_placements_are_written_back() {
    let (dir, placements) = temp_file("saved");
    let dp1 = OutputKey::of(Some("DP-1"));
    let seeded = [at(WidgetKind::Battery, 0, 0), at(WidgetKind::UpNext, 3, 2)];
    placements
        .save(&dp1, &seeded)
        .unwrap_or_else(|e| panic!("seeding: {e}"));
    let run = start_in(dir, placements.clone(), ShellFile::default());
    let h = &run.harness;
    assert_eq!(box_of(h, "#widget-battery"), (20, 16, 164, 164));
    assert_eq!(box_of(h, "#widget-calendar"), (1100, 16, 164, 164));
    assert_eq!(
        placements.load(&dp1),
        vec![
            at(WidgetKind::Battery, 0, 0),
            at(WidgetKind::UpNext, 3, 2),
            at(WidgetKind::Calendar, 6, 0),
        ],
        "the new one saved, the hidden widget's kept"
    );
}

#[test]
fn the_cell_and_the_gap_are_settings() {
    let mut file = ShellFile::default();
    file.widgets.desktop_cell_px = ds_settings::Px(200);
    file.widgets.desktop_gap_px = ds_settings::Px(0);
    file.widgets.desktop_widgets = vec![WidgetKind::Battery];
    let Running { harness: h, .. } = start("cell", file);
    // Six 200 px cells take 1200, so the grid starts at 80.
    assert_eq!(box_of(&h, "#widget-battery"), (1080, 0, 200, 200));
}

/// A widget Edit Widgets adds is simply there, at its place: nothing rises in (quire design/30
/// section 1.3), the widgets already there stay as they were, and the layer then paints nothing.
#[test]
fn an_added_widget_appears_at_its_place_and_the_layer_goes_quiet() {
    use ds_harness::harness::assert_settles_to_zero_frames;
    let Running {
        harness: mut h,
        settings: tx,
        ..
    } = start("rise", ShellFile::default());
    settle_until_two(&mut h);
    assert_settles_to_zero_frames(&mut h);

    tx.send_modify(|file| file.widgets.desktop_widgets.push(WidgetKind::WorldClock));
    h.advance(Duration::from_millis(16));
    assert_eq!(h.count("#widget-world-clock"), 1);
    assert_eq!(
        h.count(".sill-dw-place [*|data-reveal]"),
        0,
        "no reveal wrapper"
    );
    assert_settles_to_zero_frames(&mut h);
}

fn settle_until_two(h: &mut ds_harness::Harness) {
    ds_harness::harness::settle_until(h, |h| h.count(".ds-widget") == 2);
}

/// A widget Edit Widgets takes off the desktop plays its exit where it stood (quire's
/// exit, `data-presence="leaving"`), then is dropped, and the layer paints nothing; one
/// added back while it leaves is placed again and never goes.
#[test]
fn a_removed_widget_plays_its_exit_then_goes() {
    use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
    let Running {
        harness: mut h,
        settings: tx,
        ..
    } = start("leave", ShellFile::default());
    settle_until(&mut h, |h| h.count(".ds-widget") == 2);
    assert_settles_to_zero_frames(&mut h);
    let stood = box_of(&h, "#widget-battery");

    tx.send_modify(|file| {
        file.widgets
            .desktop_widgets
            .retain(|k| *k != WidgetKind::Battery)
    });
    h.advance(Duration::from_millis(16));
    assert_eq!(
        h.attr("#widget-battery", "data-presence").as_deref(),
        Some("leaving"),
        "the card stays, leaving: {}",
        h.html()
    );
    assert_eq!(
        box_of(&h, "#widget-battery"),
        stood,
        "it leaves from where it stood"
    );
    assert_eq!(
        h.attr("#widget-calendar", "data-presence").as_deref(),
        Some("present")
    );
    assert!(h.is_animating(), "the exit is playing");
    settle_until(&mut h, |h| h.count("#widget-battery") == 0);
    assert_eq!(h.count(".ds-widget"), 1);
    assert_settles_to_zero_frames(&mut h);

    // Taken away and added back before its exit ends: it stays.
    tx.send_modify(|file| {
        file.widgets
            .desktop_widgets
            .retain(|k| *k != WidgetKind::Calendar)
    });
    h.advance(Duration::from_millis(60));
    assert_eq!(
        h.attr("#widget-calendar", "data-presence").as_deref(),
        Some("leaving")
    );
    tx.send_modify(|file| file.widgets.desktop_widgets.push(WidgetKind::Calendar));
    h.advance(Duration::from_millis(16));
    assert_eq!(
        h.attr("#widget-calendar", "data-presence").as_deref(),
        Some("present")
    );
    assert_settles_to_zero_frames(&mut h);
    assert_eq!(
        h.count("#widget-calendar"),
        1,
        "it was taken back, not dropped"
    );
}
