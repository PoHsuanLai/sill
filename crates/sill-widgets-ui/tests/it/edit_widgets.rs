//! Edit Widgets mounted as a `Surface` on the virtual clock and the fake services: in the
//! gallery an added widget is written as
//! the same keys and places sill keeps (through a fake writer that publishes them back, as the
//! daemon's does, `widgets.desktop` turned on by an add to the desktop) and shown by quire's
//! gallery, its Add button settling to "Added" with a check (nothing said under the
//! gallery), a kind already there is refused with a line shaken once, a removal writes the keys
//! and says so, the gallery is a sheet at the bottom offering one size per widget and no size
//! control, and Done closes it; every moment settles to zero frames.

use ds::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{ClassPresence, Harness, Input, Viewport};
use shell_host::{OutputId, OutputInfo};
use sill_settings::{DesktopWidgets, ShellFile, WidgetKind};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{
    EditShown, EditWidgetsLink, PlacementFile, WidgetsChange, WidgetsWrite, WidgetsWriter,
};
use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
use sill_widgets_ui::EditWidgets;
use tokio::sync::watch;

/// The surface, the link it follows and every write it made.
struct Running {
    harness: Harness,
    settings: Arc<Mutex<Option<watch::Sender<ShellFile>>>>,
    edit: EditWidgetsLink,
    writes: Arc<Mutex<Vec<WidgetsWrite>>>,
    _feeds: fake::Feeds,
}

impl Running {
    fn writes(&self) -> Vec<WidgetsWrite> {
        self.writes.lock().map(|w| w.clone()).unwrap_or_default()
    }

    /// `widgets.desktop_widgets` as the surface's settings hold it now.
    fn desktop_widgets(&self) -> Vec<WidgetKind> {
        self.settings
            .lock()
            .ok()
            .and_then(|held| {
                held.as_ref()
                    .map(|tx| tx.borrow().widgets.desktop_widgets.clone())
            })
            .unwrap_or_default()
    }

    fn click(&mut self, selector: &str) {
        let at = self
            .harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is laid out: {}", self.harness.html()));
        self.harness.send(Input::click(at));
        self.harness.advance(Duration::from_millis(50));
    }
}

/// A scratch state file no other test, and no other call, shares: tests run side by side in one
/// process, and a file named only by `name` and the process would be cleared and written by every
/// test that opens a sheet at once.
fn scratch_places(name: &str) -> PlacementFile {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sill-ew-{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    PlacementFile(dir.join("desktop-widgets.json"))
}

fn start(file: ShellFile) -> Running {
    start_in(file, scratch_places("sheet"), fake::output())
}

/// The sheet of `output` over `places`: one over the same file as another's is a second
/// monitor's.
fn start_in(file: ShellFile, places: PlacementFile, output: OutputInfo) -> Running {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let log = writes.clone();
    let settings: Arc<Mutex<Option<watch::Sender<ShellFile>>>> = Arc::default();
    let publish = settings.clone();
    let saved = places.clone();
    // The daemon's writer saves the keys and the output's list and places and publishes the
    // keys; this one records and does the same.
    let writer = WidgetsWriter(Arc::new(move |change: WidgetsChange| {
        let WidgetsChange::Edit(change) = change else {
            return;
        };
        if let Ok(held) = publish.lock()
            && let Some(tx) = held.as_ref()
        {
            tx.send_modify(|file| {
                file.widgets.center = change.center.clone();
                file.widgets.desktop = change.shown;
            });
        }
        saved
            .save_layout(&change.output, &change.desktop, &change.places)
            .unwrap_or_else(|e| panic!("saving: {e}"));
        if let Ok(mut log) = log.lock() {
            log.push(change);
        }
    }));
    let viewport = Viewport {
        width: 1280,
        height: 800,
        scale_percent: 100,
    };
    let SurfaceHarness {
        mut harness,
        feeds,
        link,
        settings: tx,
        ..
    } = SurfaceHarness::<EditWidgets>::setup(
        EditWidgetsLink::new()
            .with_places(places)
            .with_writer(writer),
        (),
    )
    .output(output)
    .settings(file)
    .viewport(viewport)
    .start();
    if let Ok(mut held) = settings.lock() {
        *held = Some(tx);
    }
    harness.advance(Duration::from_millis(60));
    Running {
        harness,
        settings,
        edit: link,
        writes,
        _feeds: feeds,
    }
}

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

/// The gallery's list entry for the widget named `name`.
fn kind_option(h: &Harness, name: &str) -> String {
    let count = h.count(".ds-widget-gallery .ds-list-item");
    (1..=count)
        .map(|n| format!(".ds-widget-gallery .ds-list-item:nth-child({n}) .ds-row"))
        .find(|selector| {
            h.text_of(&format!("{selector} .ds-row-title"))
                .is_some_and(|text| text == name)
        })
        .unwrap_or_else(|| panic!("{name} is listed"))
}

const ADD_TO_DESKTOP: &str = ".ds-widget-gallery-actions .ds-button:first-child";

/// Placed rows on `host` (`desktop` or `tile`).
fn rows(h: &Harness, host: &str) -> usize {
    h.count(&format!(
        ".ds-widget-gallery-surface[*|data-host={host}] .ds-widget-gallery-row"
    ))
}

/// Open the gallery and wait until it has measured the desktop's grid (the desktop's widgets
/// are listed once it knows their cells).
fn opened() -> Running {
    let mut run = start(ShellFile::default());
    open_with_desktop_rows(&mut run, fake::output().id, 2);
    run
}

/// Open `run`'s gallery on `output` and wait for `desktop_rows` placed rows on the desktop.
fn open_with_desktop_rows(run: &mut Running, output: OutputId, desktop_rows: usize) {
    assert_eq!(
        run.harness.count(".ds-widget-gallery"),
        0,
        "hidden until opened"
    );
    run.edit.open(output);
    settle_until(&mut run.harness, |h| rows(h, "desktop") == desktop_rows);
    // The panel has slid in and holds still before anything is pressed.
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn an_added_widget_is_written_as_the_keys_and_places_and_a_second_is_refused() {
    let mut run = opened();
    assert_eq!(run.harness.count(".ds-widget-gallery .ds-list-item"), 5);
    assert_eq!(
        (rows(&run.harness, "desktop"), rows(&run.harness, "tile")),
        (2, 3),
        "the defaults: the month and the batteries; the month, Up Next and the batteries"
    );
    let clock = kind_option(&run.harness, "World Clock");
    run.click(&clock);
    run.click(ADD_TO_DESKTOP);
    // 1280 x 800 holds 7 x 4 cells; the month and the batteries sit at the top right, so a
    // medium's first free spot scanning from the right is column 5, row 2.
    assert_eq!(
        run.writes(),
        vec![WidgetsWrite {
            center: vec![
                WidgetKind::Calendar,
                WidgetKind::UpNext,
                WidgetKind::Battery
            ],
            desktop: vec![
                WidgetKind::Calendar,
                WidgetKind::Battery,
                WidgetKind::WorldClock
            ],
            shown: DesktopWidgets::On,
            output: OutputKey::Named("DP-1".to_owned()),
            places: vec![
                at(WidgetKind::Calendar, 6, 0),
                at(WidgetKind::Battery, 6, 1),
                at(WidgetKind::WorldClock, 5, 2),
            ],
        }]
    );
    // quire's gallery shows no check for the add (the success beat is gone): the button reads as
    // before, and sill says nothing more under the gallery.
    assert_eq!(
        run.harness.text_of(ADD_TO_DESKTOP).as_deref(),
        Some("Add to Desktop")
    );
    assert_eq!(status(&run.harness), None, "no line under the gallery");
    settle_until(&mut run.harness, |h| rows(h, "desktop") == 3);
    assert_eq!(run.harness.count(".ds-widget-gallery-added"), 0);
    assert_settles_to_zero_frames(&mut run.harness);

    run.click(&clock);
    run.click(ADD_TO_DESKTOP);
    assert_eq!(run.writes().len(), 1, "nothing written");
    assert_eq!(
        status(&run.harness),
        Some(("refused".to_owned(), "It is already there.".to_owned()))
    );
    assert_eq!(
        run.harness.has_class(".sill-ew-status", "a-shake-x"),
        ClassPresence::Present,
        "a refusal shakes once: {}",
        run.harness.html()
    );
    assert_eq!(
        run.harness.count(".ds-widget-gallery-added"),
        0,
        "no check for a refusal"
    );
    settle_until(&mut run.harness, |h| {
        h.has_class(".sill-ew-status", "a-shake-x") == ClassPresence::Absent
    });
    assert_settles_to_zero_frames(&mut run.harness);

    run.click(".sill-ew-head .ds-button");
    assert_eq!(run.edit.now(), EditShown::Closed);
    settle_until(&mut run.harness, |h| h.count(".ds-sheet") == 0);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_removed_tile_leaves_the_center_key() {
    let mut run = opened();
    let up_next = ".ds-widget-gallery-surface[*|data-host=tile] \
                   .ds-widget-gallery-row:nth-child(3) .ds-button[*|data-role=destructive]";
    run.click(up_next);
    let writes = run.writes();
    assert_eq!(
        writes.last().map(|w| w.center.clone()),
        Some(vec![WidgetKind::Calendar, WidgetKind::Battery])
    );
    assert_eq!(
        writes.last().map(|w| w.desktop.clone()),
        Some(vec![WidgetKind::Calendar, WidgetKind::Battery]),
        "the desktop is untouched"
    );
    assert_eq!(
        writes.last().map(|w| w.shown),
        Some(DesktopWidgets::Off),
        "a removal leaves the desktop's widgets as they were"
    );
    assert_eq!(
        status(&run.harness),
        Some((
            "removed".to_owned(),
            "Removed Up Next from the Notification Center.".to_owned()
        ))
    );
    assert_eq!(
        run.harness.count(".ds-widget-gallery-added"),
        0,
        "a removal draws no check"
    );
    assert_settles_to_zero_frames(&mut run.harness);
}

/// The gallery is a sheet at the bottom of the output, under half its height, and
/// offers each widget at its one size: one preview, no size control, Add at that size.
#[test]
fn the_gallery_is_a_bottom_sheet_offering_one_size_per_widget() {
    let mut run = opened();
    let h = &run.harness;
    assert_eq!(
        h.attr(".ds-sheet", "data-attach").as_deref(),
        Some("bottom")
    );
    let (left, top, width, height) = box_of(h, ".ds-sheet");
    assert!(top + height >= 800 - 8 - 1, "at the bottom: {top} {height}");
    assert!(height <= 400, "under half the output: {height}");
    assert_eq!(width, 1040, "{left} {width}");
    assert_eq!(left, (1280 - 1040) / 2, "centred");
    // Batteries last: the entry the gallery opens on, whose preview runs no clock.
    for name in [
        "Calendar",
        "World Clock",
        "Up Next",
        "Now Playing",
        "Batteries",
    ] {
        let entry = kind_option(&run.harness, name);
        run.click(&entry);
        let h = &run.harness;
        assert_eq!(
            h.count(".ds-widget-gallery-preview"),
            1,
            "{name}: one preview"
        );
        assert_eq!(
            h.count(".ds-widget-gallery-preview .ds-widget"),
            1,
            "{name}"
        );
        let want = match name {
            "Calendar" | "Batteries" => "small",
            _ => "medium",
        };
        assert_eq!(
            h.attr(".ds-widget-gallery-preview", "data-size").as_deref(),
            Some(want),
            "{name} at its desktop size"
        );
    }
    let h = &run.harness;
    assert_eq!(
        h.count(".ds-widget-gallery .ds-segmented")
            + h.count(".ds-widget-gallery .ds-segmented-segment"),
        0,
        "no size control anywhere: {}",
        h.html()
    );
    for size in ["Small", "Medium", "Large"] {
        assert!(
            !h.html().contains(&format!(">{size}<")),
            "no size named in the gallery: {size}"
        );
    }
    assert_settles_to_zero_frames(&mut run.harness);
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
fn nothing_is_said_until_something_is_edited() {
    let run = opened();
    assert_eq!(status(&run.harness), None);
    assert_eq!(run.harness.count(".sill-ew-status"), 0);
}

/// The status line's outcome and words, when there is one.
fn status(h: &Harness) -> Option<(String, String)> {
    let outcome = h.attr(".sill-ew-status", "data-outcome")?;
    let words = h.text_of(".sill-ew-status .ds-label")?;
    Some((outcome, words))
}

/// Escape closes the panel after a press inside it (a list entry, Add, Remove), not only
/// right after it opens.
#[test]
fn escape_closes_after_a_press_inside_the_panel() {
    /// What is pressed after World Clock's list entry.
    #[derive(Debug, Clone, Copy)]
    enum Then {
        Nothing,
        AddToDesktop,
        RemoveADesktopRow,
    }
    for then in [Then::Nothing, Then::AddToDesktop, Then::RemoveADesktopRow] {
        let mut run = opened();
        let clock = kind_option(&run.harness, "World Clock");
        run.click(&clock);
        match then {
            Then::Nothing => {}
            Then::AddToDesktop => run.click(ADD_TO_DESKTOP),
            Then::RemoveADesktopRow => run.click(
                ".ds-widget-gallery-surface[*|data-host=desktop] \
                 .ds-widget-gallery-row:nth-child(3) .ds-button[*|data-role=destructive]",
            ),
        }
        run.harness.advance(Duration::from_millis(100));
        run.harness.send(Input::key(ShortcutKey::Escape));
        run.harness.advance(Duration::from_millis(50));
        assert_eq!(run.edit.now(), EditShown::Closed, "after {then:?}");
        settle_until(&mut run.harness, |h| h.count(".ds-sheet") == 0);
        assert_settles_to_zero_frames(&mut run.harness);
    }
}

/// A state file where `HDMI-A-1` has a list of its own and `DP-1` follows the setting.
fn two_monitors(name: &str) -> PlacementFile {
    let places = scratch_places(name);
    places
        .save_layout(
            &OutputKey::of(Some("HDMI-A-1")),
            &[WidgetKind::Battery],
            &[at(WidgetKind::Battery, 6, 0)],
        )
        .unwrap_or_else(|e| panic!("seeding: {e}"));
    places
}

/// The widgets each monitor's own list holds in the file.
fn lists(places: &PlacementFile) -> (Option<Vec<WidgetKind>>, Option<Vec<WidgetKind>>) {
    (
        places.kinds(&OutputKey::of(Some("DP-1"))),
        places.kinds(&OutputKey::of(Some("HDMI-A-1"))),
    )
}

#[test]
fn each_monitors_sheet_lists_and_edits_its_own_desktop_widgets() {
    let places = two_monitors("per-output");
    let hdmi = fake::output_named(Some("HDMI-A-1"));
    // HDMI-A-1's own list is one widget; the sheet lists that one, not the setting's two.
    let mut second = start_in(ShellFile::default(), places.clone(), hdmi.clone());
    open_with_desktop_rows(&mut second, hdmi.id, 1);
    assert_eq!(lists(&places), (None, Some(vec![WidgetKind::Battery])));
    let clock = kind_option(&second.harness, "World Clock");
    second.click(&clock);
    second.click(ADD_TO_DESKTOP);
    let writes = second.writes();
    assert_eq!(writes.len(), 1, "one edit, one write");
    assert_eq!(
        (writes[0].output.clone(), writes[0].desktop.clone()),
        (
            OutputKey::of(Some("HDMI-A-1")),
            vec![WidgetKind::Battery, WidgetKind::WorldClock]
        ),
        "the edit is HDMI-A-1's"
    );
    assert_eq!(
        lists(&places),
        (
            None,
            Some(vec![WidgetKind::Battery, WidgetKind::WorldClock])
        ),
        "DP-1 still follows the setting"
    );
    settle_until(&mut second.harness, |h| rows(h, "desktop") == 2);

    // The first monitor's sheet over the same file lists the setting's two widgets; removing one
    // writes its own list and leaves HDMI-A-1's.
    let mut first = start_in(ShellFile::default(), places.clone(), fake::output());
    open_with_desktop_rows(&mut first, fake::output().id, 2);
    first.click(
        ".ds-widget-gallery-surface[*|data-host=desktop] \
         .ds-widget-gallery-row:nth-child(2) .ds-button[*|data-role=destructive]",
    );
    assert_eq!(
        lists(&places),
        (
            Some(vec![WidgetKind::Battery]),
            Some(vec![WidgetKind::Battery, WidgetKind::WorldClock])
        ),
        "removing the month from DP-1 leaves its batteries, and HDMI-A-1's list as it was"
    );
    assert_eq!(
        first.writes().last().map(|w| w.output.clone()),
        Some(OutputKey::of(Some("DP-1")))
    );
}

#[test]
fn an_edit_of_one_monitor_does_not_touch_the_setting_the_others_follow() {
    let places = two_monitors("setting");
    let hdmi = fake::output_named(Some("HDMI-A-1"));
    let mut run = start_in(ShellFile::default(), places.clone(), hdmi.clone());
    open_with_desktop_rows(&mut run, hdmi.id, 1);
    let clock = kind_option(&run.harness, "World Clock");
    run.click(&clock);
    run.click(ADD_TO_DESKTOP);
    assert_eq!(
        run.desktop_widgets(),
        ShellFile::default().widgets.desktop_widgets,
        "widgets.desktop_widgets is the default list, not one monitor's"
    );
}
