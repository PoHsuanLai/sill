//! What the desktop widget layer draws (M10 lane b), the real `DesktopWidgets` in a
//! `ds_native::Harness` on the fake services: `widgets.desktop_widgets` placed on the grid from
//! the top right, each in its Widget-material frame at its cell's box; a saved placement kept;
//! the placements written back to the file the root was given (a temp file here: never the
//! user's state); and nothing painted between the widgets.

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_surfaces::DesktopWidgets;
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::surfaces::desktop_widgets::{GridSlot, OutputKey, Placement, PlacementFile};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    file: Option<PlacementFile>,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    if let Some(file) = world.file {
        use_context_provider(|| file);
    }
    rsx! {
        DesktopWidgets { output: fake::output() }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn start(file: ShellFile, placements: Option<PlacementFile>) -> (Harness, fake::Feeds) {
    let (services, feeds) = fake::services();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            file: placements,
        });
    });
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    (harness, feeds)
}

fn temp_file(name: &str) -> (PathBuf, PlacementFile) {
    let dir = std::env::temp_dir().join(format!("sill-dw-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = PlacementFile(dir.join("desktop-widgets.json"));
    (dir, file)
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
    let (h, _feeds) = start(ShellFile::default(), None);
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
    let (h, _feeds) = start(file, None);
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
    let (h, _feeds) = start(ShellFile::default(), Some(placements.clone()));
    assert_eq!(box_of(&h, "#widget-battery"), (20, 16, 164, 164));
    assert_eq!(box_of(&h, "#widget-calendar"), (1100, 16, 164, 164));
    assert_eq!(
        placements.load(&dp1),
        vec![
            at(WidgetKind::Battery, 0, 0),
            at(WidgetKind::UpNext, 3, 2),
            at(WidgetKind::Calendar, 6, 0),
        ],
        "the new one saved, the hidden widget's kept"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_cell_and_the_gap_are_settings() {
    let mut file = ShellFile::default();
    file.widgets.desktop_cell_px = ds_settings::Px(200);
    file.widgets.desktop_gap_px = ds_settings::Px(0);
    file.widgets.desktop_widgets = vec![WidgetKind::Battery];
    let (h, _feeds) = start(file, None);
    // Six 200 px cells take 1200, so the grid starts at 80.
    assert_eq!(box_of(&h, "#widget-battery"), (1080, 0, 200, 200));
}

/// F1025: a widget Edit Widgets adds arrives visibly: its card rises into its place once
/// (quire's `Reveal`, design/26 Appear), the widgets already there do not replay theirs, and the
/// layer then paints nothing.
#[test]
fn an_added_widget_rises_into_its_place_once() {
    use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
    let (services, _feeds) = fake::services();
    let (tx, rx) = tokio::sync::watch::channel(ShellFile::default());
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed(rx),
            file: None,
        });
    });
    let mut h = Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    h.advance(Duration::from_millis(40));
    settle_until(&mut h, |h| h.count(".ds-widget") == 2);
    settle_until(&mut h, |h| {
        h.count(".sill-dw-place [*|data-reveal=play]") == 0
    });
    assert_settles_to_zero_frames(&mut h);

    tx.send_modify(|file| file.widgets.desktop_widgets.push(WidgetKind::WorldClock));
    h.advance(Duration::from_millis(16));
    assert_eq!(h.count("#widget-world-clock"), 1);
    assert_eq!(
        h.count(".sill-dw-place [*|data-reveal=play]"),
        1,
        "only the new card rises: {}",
        h.html()
    );
    assert!(h.is_animating(), "the rise is playing");
    settle_until(&mut h, |h| {
        h.count(".sill-dw-place [*|data-reveal=play]") == 0
    });
    assert_eq!(h.count(".sill-dw-place [*|data-reveal=still]"), 3);
    assert_settles_to_zero_frames(&mut h);
}

/// A desktop widget's harness on a settings feed the test can change.
fn start_live() -> (Harness, tokio::sync::watch::Sender<ShellFile>, fake::Feeds) {
    let (services, feeds) = fake::services();
    let (tx, rx) = tokio::sync::watch::channel(ShellFile::default());
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed(rx),
            file: None,
        });
    });
    let mut h = Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    h.advance(Duration::from_millis(40));
    (h, tx, feeds)
}

/// G423: a widget Edit Widgets takes off the desktop plays its exit where it stood (quire's
/// `widget-out`, `data-presence="leaving"`), then is dropped, and the layer paints nothing; one
/// added back while it leaves is placed again and never goes.
#[test]
fn a_removed_widget_plays_its_exit_then_goes() {
    use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
    let (mut h, tx, _feeds) = start_live();
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
    assert!(
        h.has_class("#widget-battery", "a-widget-out"),
        "{}",
        h.html()
    );
    assert_eq!(
        box_of(&h, "#widget-battery"),
        stood,
        "it leaves from where it stood"
    );
    assert_eq!(h.attr("#widget-calendar", "data-presence"), None);
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
    assert_eq!(h.attr("#widget-calendar", "data-presence"), None);
    assert_settles_to_zero_frames(&mut h);
    assert_eq!(
        h.count("#widget-calendar"),
        1,
        "it was taken back, not dropped"
    );
}
