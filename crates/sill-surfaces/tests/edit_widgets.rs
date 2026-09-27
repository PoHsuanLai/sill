//! Edit Widgets (FINDINGS "Widget interface port"), the real surfaces in a `ds_native::Harness`
//! on the virtual clock and the fake services: the notification center's "Edit Widgets" closes
//! the center and opens the gallery on its output; in the gallery an added widget is written as
//! the same keys and places sill keeps (through a fake writer that publishes them back, as the
//! daemon's does), a kind already there is refused with a line saying so, a size change and a
//! removal write the keys, and Done closes it; every moment settles to zero frames.

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::surfaces::desktop_widgets::{GridSlot, OutputKey, Placement};
use sill_surfaces::surfaces::edit_widgets::{
    EditShown, EditWidgetsLink, WidgetsWrite, WidgetsWriter,
};
use sill_surfaces::surfaces::notification_center::{CenterLink, CenterShown};
use sill_surfaces::{EditWidgets, NotificationCenter};
use tokio::sync::watch;

/// Which surface the harness draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawn {
    Center,
    Edit,
}

#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    center: CenterLink,
    edit: EditWidgetsLink,
    writer: WidgetsWriter,
    drawn: Drawn,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let Some(world) = WORLD.with(|w| w.borrow().clone()) else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.center.clone());
    use_context_provider(|| world.edit.clone());
    use_context_provider(|| world.writer.clone());
    match world.drawn {
        Drawn::Center => rsx! { NotificationCenter { output: fake::output() } },
        Drawn::Edit => rsx! { EditWidgets { output: fake::output() } },
    }
}

/// A running surface, the links it follows and every write it made.
struct Running {
    harness: Harness,
    center: CenterLink,
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

fn start(drawn: Drawn, file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    let (tx, rx) = watch::channel(file);
    let writes = Arc::new(Mutex::new(Vec::new()));
    let log = writes.clone();
    // The daemon's writer saves the keys and publishes them; this one records and publishes.
    let writer = WidgetsWriter(Arc::new(move |change: WidgetsWrite| {
        tx.send_modify(|file| {
            file.widgets.center = change.center.clone();
            file.widgets.desktop_widgets = change.desktop.clone();
            file.widgets.sizes = change.sizes.clone();
        });
        if let Ok(mut log) = log.lock() {
            log.push(change);
        }
    }));
    let center = CenterLink::new();
    let edit = EditWidgetsLink::new();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed(rx),
            center: center.clone(),
            edit: edit.clone(),
            writer,
            drawn,
        });
    });
    let viewport = Viewport {
        width: 1280,
        height: 800,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(60));
    Running {
        harness,
        center,
        edit,
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
    let mut run = start(Drawn::Edit, ShellFile::default());
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
fn the_centers_edit_widgets_opens_the_gallery_on_its_output() {
    // One tile, so the button is in view without scrolling the column.
    let mut file = ShellFile::default();
    file.widgets.center = vec![WidgetKind::Battery];
    let mut run = start(Drawn::Center, file);
    let output = fake::output().id;
    run.center.open(output);
    settle_until(&mut run.harness, |h| {
        h.count(".sill-nc-edit .ds-button") == 1
    });
    assert_eq!(
        run.harness.text_of(".sill-nc-edit .ds-button").as_deref(),
        Some("Edit Widgets")
    );
    let widgets = run.harness.rect(".sill-nc-widgets").expect("the widgets");
    let button = run.harness.rect(".sill-nc-edit").expect("the button");
    assert!(
        button.origin.y.0
            >= widgets.origin.y.0 + widgets.size.height.0 - button.size.height.0 - 0.5,
        "at the bottom of the widgets: {widgets:?} {button:?}"
    );
    assert_settles_to_zero_frames(&mut run.harness);
    run.click(".sill-nc-edit .ds-button");
    assert_eq!(run.center.now(), CenterShown::Closed);
    assert_eq!(run.edit.now(), EditShown::Open(output));
    settle_until(&mut run.harness, |h| h.count(".ds-panel") == 0);
    assert_settles_to_zero_frames(&mut run.harness);
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
            sizes: String::new(),
            output: OutputKey::Named("DP-1".to_owned()),
            places: vec![
                at(WidgetKind::Calendar, 6, 0),
                at(WidgetKind::Battery, 6, 1),
                at(WidgetKind::WorldClock, 5, 2),
            ],
        }]
    );
    settle_until(&mut run.harness, |h| rows(h, "desktop") == 3);
    assert_eq!(run.harness.count(".sill-ew-refused"), 0);

    run.click(&clock);
    run.click(ADD_TO_DESKTOP);
    assert_eq!(run.writes().len(), 1, "nothing written");
    assert_eq!(
        run.harness.text_of(".sill-ew-refused").as_deref(),
        Some("It is already there.")
    );
    assert_settles_to_zero_frames(&mut run.harness);

    run.click(".sill-ew-head .ds-button");
    assert_eq!(run.edit.now(), EditShown::Closed);
    settle_until(&mut run.harness, |h| h.count(".ds-panel") == 0);
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_removed_tile_leaves_the_center_key() {
    let mut run = opened();
    let up_next = ".ds-widget-gallery-surface[*|data-host=tile] \
                   .ds-widget-gallery-row:nth-child(3) .ds-button[*|data-variant=danger]";
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
    assert_settles_to_zero_frames(&mut run.harness);
}

#[test]
fn a_size_picked_for_a_desktop_widget_is_written_to_the_sizes_key() {
    let mut run = opened();
    // The desktop's first row is the month (row 0 of the grid); its sizes are Small, Medium
    // and Large, so the second segment is Medium.
    let medium = ".ds-widget-gallery-surface[*|data-host=desktop] \
                  .ds-widget-gallery-row:nth-child(2) .ds-segment:nth-child(2)";
    run.click(medium);
    let last = run.writes().last().cloned().expect("a write");
    assert_eq!(last.sizes, "desktop.calendar=medium");
    assert_eq!(
        last.places.first(),
        Some(&at(WidgetKind::Calendar, 5, 0)),
        "a medium no longer fits at the right edge: the first free spot, a column in"
    );
    assert_settles_to_zero_frames(&mut run.harness);
}
