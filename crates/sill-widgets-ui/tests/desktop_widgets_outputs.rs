//! What each monitor's desktop layer shows, the real `DesktopWidgets` mounted as a `Surface` on
//! the fake services: the widgets its own output's list in the state file names, else the
//! setting's list; another monitor's list changes nothing here; and an Edit Widgets write
//! through the link is read again, so the layer follows its sheet. A World Clock on the desktop
//! names the date of a city whose day is not today's.

use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::use_context_provider;
use ds::prelude::Word;
use ds_harness::{Driver, Harness, Query, Viewport};
use shell_host::OutputInfo;
use sill_settings::{ShellFile, WidgetKind};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{
    EditWidgetsLink, PlacementFile, WidgetsChange, WidgetsWrite, WidgetsWriter,
};
use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
use std::sync::Arc;

use sill_widgets_ui::DesktopWidgets;

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running layer and the link its sheet's writes go through.
struct Running {
    harness: Harness,
    link: EditWidgetsLink,
    _scratch: Scratch,
}

/// A scratch directory, removed when the test ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

/// The state file of the test named `name`, with `HDMI-A-1` keeping `Battery` alone.
fn state(name: &str) -> (PathBuf, PlacementFile) {
    let dir = std::env::temp_dir().join(format!("sill-dw-out-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let places = PlacementFile(dir.join("desktop-widgets.json"));
    places
        .save_layout(
            &OutputKey::of(Some("HDMI-A-1")),
            &[WidgetKind::Battery],
            &[at(WidgetKind::Battery, 6, 0)],
        )
        .unwrap_or_else(|e| panic!("seeding: {e}"));
    (dir, places)
}

/// The layer of `output` over `places`, its Edit Widgets link writing the way the daemon's does.
fn start(dir: PathBuf, places: PlacementFile, output: OutputInfo) -> Running {
    let saved = places.clone();
    let writer = WidgetsWriter(Arc::new(move |change: WidgetsChange| {
        if let WidgetsChange::Edit(change) = change {
            saved
                .save_layout(&change.output, &change.desktop, &change.places)
                .unwrap_or_else(|e| panic!("saving: {e}"));
        }
    }));
    let link = EditWidgetsLink::new()
        .with_places(places.clone())
        .with_writer(writer);
    let provided = link.clone();
    let SurfaceHarness { mut harness, .. } = SurfaceHarness::<DesktopWidgets>::setup(places, ())
        .output(output)
        .settings(ShellFile::default())
        .viewport(VIEWPORT)
        .in_root(move || {
            let link = provided.clone();
            use_context_provider(|| link);
        })
        .start();
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    Running {
        harness,
        link,
        _scratch: Scratch(dir),
    }
}

/// The widgets the layer draws, by element id.
fn drawn(h: &Harness) -> Vec<String> {
    WidgetKind::ALL
        .iter()
        .map(|kind| format!("widget-{}", kind.slug()))
        .filter(|id| h.count(&format!("#{id}")) == 1)
        .collect()
}

#[test]
fn a_monitor_draws_its_own_list_and_another_follows_the_setting() {
    let (dir, places) = state("own-list");
    let hdmi = start(
        dir.clone(),
        places.clone(),
        fake::output_named(Some("HDMI-A-1")),
    );
    assert_eq!(
        drawn(&hdmi.harness),
        ["widget-battery"],
        "HDMI-A-1's own list"
    );
    let dp1 = start(dir, places, fake::output());
    assert_eq!(
        drawn(&dp1.harness),
        ["widget-calendar", "widget-battery"],
        "DP-1 has no list: the setting's"
    );
}

#[test]
fn an_edit_written_for_this_output_is_drawn_and_one_for_another_is_not() {
    let (dir, places) = state("follows");
    let mut run = start(dir, places, fake::output());
    assert_eq!(drawn(&run.harness), ["widget-calendar", "widget-battery"]);
    let write = |output: &str, desktop: Vec<WidgetKind>| WidgetsWrite {
        center: vec![WidgetKind::Calendar],
        desktop,
        shown: sill_settings::DesktopWidgets::On,
        output: OutputKey::of(Some(output)),
        places: vec![at(WidgetKind::WorldClock, 5, 2)],
    };
    run.link.write(WidgetsChange::Edit(write(
        "HDMI-A-1",
        vec![WidgetKind::WorldClock],
    )));
    for _ in 0..5 {
        run.harness.advance(Duration::from_millis(40));
    }
    assert_eq!(
        drawn(&run.harness),
        ["widget-calendar", "widget-battery"],
        "another monitor's edit changes nothing here"
    );
    run.link.write(WidgetsChange::Edit(write(
        "DP-1",
        vec![WidgetKind::WorldClock, WidgetKind::Battery],
    )));
    for _ in 0..10 {
        run.harness.advance(Duration::from_millis(40));
    }
    assert_eq!(
        drawn(&run.harness),
        ["widget-battery", "widget-world-clock"],
        "this monitor's own edit is read again"
    );
}

#[test]
fn a_world_clock_names_the_date_of_a_city_on_another_day() {
    // Kiritimati is 25 hours ahead of Pago Pago, so at any moment their dates differ, and at
    // least one of them is not the date here: its first note is a date, `Sat 27 Sep`.
    let (dir, places) = state("dates");
    let mut file = ShellFile::default();
    file.widgets.desktop_widgets = vec![WidgetKind::WorldClock];
    file.widgets.world_clocks = "Pacific/Kiritimati; Pacific/Pago_Pago".to_owned();
    let SurfaceHarness { mut harness, .. } = SurfaceHarness::<DesktopWidgets>::setup(places, ())
        .settings(file)
        .viewport(VIEWPORT)
        .start();
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    let _scratch = Scratch(dir);
    // Each city's div holds its dial, then its notes: the day, then the offset.
    let notes: Vec<String> = (1..=2)
        .flat_map(|city| (2..=3).map(move |child| (city, child)))
        .filter_map(|(city, child)| {
            harness.text_of(&format!(
                ".ds-world-clock-city:nth-child({city}) .ds-world-clock-note:nth-child({child})"
            ))
        })
        .collect();
    assert_eq!(notes.len(), 4, "two cities, two notes each: {notes:?}");
    let is_date = |note: &str| {
        let words: Vec<&str> = note.split(' ').collect();
        matches!(words.as_slice(), [weekday, day, month]
            if weekday.len() == 3 && day.parse::<u8>().is_ok() && month.len() == 3)
    };
    let dated = notes.iter().filter(|n| is_date(n)).count();
    let today = notes.iter().filter(|n| *n == "Today").count();
    assert_eq!(
        (dated >= 1, today <= 1),
        (true, true),
        "a city on another day names its date, and only one can share ours: {notes:?}"
    );
}
