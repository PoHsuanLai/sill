//! Edit Widgets mounted as a `Surface` on the virtual clock and the fake services: in the
//! gallery an added widget is written as
//! the same keys and places sill keeps (through a fake writer that publishes them back, as the
//! daemon's does, `widgets.desktop` turned on by an add to the desktop) and shown by quire's
//! gallery, its Add button settling to "Added" with a check (nothing said under the
//! gallery), a kind already there is refused with a line shaken once, a removal writes the keys
//! and says so, the gallery is a sheet at the bottom offering one size per widget and no size
//! control, and Done closes it; every moment settles to zero frames.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use ds_harness::{Harness, Viewport};
use sill_settings::{DesktopWidgets, ShellFile, WidgetKind};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{EditShown, EditWidgetsLink, WidgetsWrite, WidgetsWriter};
use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
use sill_widgets_ui::EditWidgets;
use tokio::sync::watch;

/// The surface, the link it follows and every write it made.
struct Running {
    harness: Harness,
    edit: EditWidgetsLink,
    writes: Arc<Mutex<Vec<WidgetsWrite>>>,
    _feeds: fake::Feeds,
}

impl Running {
    fn writes(&self) -> Vec<WidgetsWrite> {
        self.writes.lock().map(|w| w.clone()).unwrap_or_default()
    }

    fn click(&mut self, selector: &str) {
        let at = self
            .harness
            .centre(selector)
            .unwrap_or_else(|| panic!("{selector} is laid out: {}", self.harness.html()));
        self.harness.click(at);
        self.harness.advance(Duration::from_millis(50));
    }
}

fn start(file: ShellFile) -> Running {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let log = writes.clone();
    let settings: Arc<Mutex<Option<watch::Sender<ShellFile>>>> = Arc::default();
    let publish = settings.clone();
    // The daemon's writer saves the keys and publishes them; this one records and publishes.
    let writer = WidgetsWriter(Arc::new(move |change: WidgetsWrite| {
        if let Ok(held) = publish.lock()
            && let Some(tx) = held.as_ref()
        {
            tx.send_modify(|file| {
                file.widgets.center = change.center.clone();
                file.widgets.desktop_widgets = change.desktop.clone();
                file.widgets.desktop = change.shown;
            });
        }
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
    } = SurfaceHarness::<EditWidgets>::setup(EditWidgetsLink::new().with_writer(writer), ())
        .settings(file)
        .viewport(viewport)
        .start();
    if let Ok(mut held) = settings.lock() {
        *held = Some(tx);
    }
    harness.advance(Duration::from_millis(60));
    Running {
        harness,
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
    let count = h.count(".ds-widget-gallery-kind");
    (1..=count)
        .map(|n| format!(".ds-widget-gallery-kind:nth-child({n})"))
        .find(|selector| {
            h.text_of(&format!("{selector} .ds-widget-gallery-name"))
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
    assert_eq!(
        run.harness.count(".ds-widget-gallery"),
        0,
        "hidden until opened"
    );
    run.edit.open(fake::output().id);
    settle_until(&mut run.harness, |h| rows(h, "desktop") == 2);
    // The panel has slid in and holds still before anything is pressed.
    assert_settles_to_zero_frames(&mut run.harness);
    run
}

#[test]
fn an_added_widget_is_written_as_the_keys_and_places_and_a_second_is_refused() {
    let mut run = opened();
    assert_eq!(run.harness.count(".ds-widget-gallery-kind"), 5);
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
    assert!(
        run.harness.has_class(".sill-ew-status", "a-shake-x"),
        "a refusal shakes once: {}",
        run.harness.html()
    );
    assert_eq!(
        run.harness.count(".ds-widget-gallery-added"),
        0,
        "no check for a refusal"
    );
    settle_until(&mut run.harness, |h| {
        !h.has_class(".sill-ew-status", "a-shake-x")
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
    let words = h.text_of(".sill-ew-status-words")?;
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
        run.harness.key(ds::ShortcutKey::Escape);
        run.harness.advance(Duration::from_millis(50));
        assert_eq!(run.edit.now(), EditShown::Closed, "after {then:?}");
        settle_until(&mut run.harness, |h| h.count(".ds-sheet") == 0);
        assert_settles_to_zero_frames(&mut run.harness);
    }
}
