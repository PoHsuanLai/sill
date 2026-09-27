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
    // A file from before places were per output: its entries name no output.
    let seeded = [at(WidgetKind::Battery, 0, 0), at(WidgetKind::UpNext, 3, 2)];
    placements
        .save(&OutputKey::Any, &seeded)
        .unwrap_or_else(|e| panic!("seeding: {e}"));
    let (h, _feeds) = start(ShellFile::default(), Some(placements.clone()));
    assert_eq!(box_of(&h, "#widget-battery"), (20, 16, 164, 164));
    assert_eq!(box_of(&h, "#widget-calendar"), (1100, 16, 164, 164));
    assert_eq!(
        placements.load(&OutputKey::of(Some("DP-1"))),
        vec![
            at(WidgetKind::Battery, 0, 0),
            at(WidgetKind::UpNext, 3, 2),
            at(WidgetKind::Calendar, 6, 0),
        ],
        "DP-1 now has places of its own: the new one saved, the hidden widget's kept"
    );
    assert_eq!(
        placements.load(&OutputKey::Any),
        seeded,
        "the unnamed entries stay for other outputs"
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
