//! Dragging a desktop widget, the real
//! `DesktopWidgets` in a `ds_native::Harness` on the virtual clock: a press that travels lifts
//! the widget under the pointer (quire's card `Lift::Lifted`, with quire's slot guide at the
//! cell it would land in), the one it passes over makes room and goes back after, a release
//! snaps it into the grid and saves the place for this output, a click still reaches a widget's
//! own control, and every moment settles to zero frames.
//!
//! The layer is 1280 x 800: seven 164 px columns from x = 20 (column `c` at 20 + 180 c), rows at
//! 16 + 180 r. The Calendar starts at column 6 row 0 (1100, 16), the Battery under it (1100, 196).

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Point, Px};
use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::now_playing::{MediaCommand, MediaVerb};
use sill_settings::{SettingsFeed, ShellFile, WidgetKind};
use sill_surfaces::DesktopWidgets;
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::desktop_widgets::{GridSlot, OutputKey, Placement, PlacementFile};

#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    file: PlacementFile,
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
    use_context_provider(|| world.file.clone());
    rsx! {
        DesktopWidgets { output: fake::output() }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

struct Running {
    harness: Harness,
    feeds: fake::Feeds,
    file: PlacementFile,
    dir: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start(name: &str, kinds: Vec<WidgetKind>) -> Running {
    let (services, feeds) = fake::services();
    let dir = std::env::temp_dir().join(format!("sill-dw-drag-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = PlacementFile(dir.join("desktop-widgets.json"));
    let mut settings = ShellFile::default();
    settings.widgets.desktop_widgets = kinds;
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(settings),
            file: file.clone(),
        });
    });
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    for _ in 0..10 {
        harness.advance(Duration::from_millis(40));
    }
    Running {
        harness,
        feeds,
        file,
        dir,
    }
}

fn point(x: f32, y: f32) -> Point {
    Point { x: Px(x), y: Px(y) }
}

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

/// The box of the place holding `id`, as whole px: (left, top, width, height).
fn box_of(h: &Harness, id: &str) -> (i32, i32, i32, i32) {
    let rect = h
        .rect(&format!("#{id}"))
        .unwrap_or_else(|| panic!("{id} is laid out"));
    let px = |v: f32| v.round() as i32;
    (
        px(rect.origin.x.0),
        px(rect.origin.y.0),
        px(rect.size.width.0),
        px(rect.size.height.0),
    )
}

fn lifted(h: &Harness, id: &str) -> usize {
    h.count(&format!(".ds-widget#{id}[*|data-lift=lifted]"))
}

/// The slot guide's box, while one is drawn.
fn guide(h: &Harness) -> Option<(i32, i32, i32, i32)> {
    let r = h.rect(".sill-dw-guide .ds-widget-slot")?;
    let px = |v: f32| v.round() as i32;
    Some((
        px(r.origin.x.0),
        px(r.origin.y.0),
        px(r.size.width.0),
        px(r.size.height.0),
    ))
}

/// The Calendar's centre, where every drag here takes it (82 px in from its corner).
const CALENDAR: (f32, f32) = (1182.0, 98.0);

/// Press the Calendar and move the pointer to `to` in a few steps, without releasing.
fn pick_up(h: &mut Harness, to: (f32, f32)) {
    let from = point(CALENDAR.0, CALENDAR.1);
    h.pointer_move(from);
    h.pointer_down(from);
    for step in 1..=4 {
        let part = step as f32 / 4.0;
        h.pointer_move(point(
            CALENDAR.0 + (to.0 - CALENDAR.0) * part,
            CALENDAR.1 + (to.1 - CALENDAR.1) * part,
        ));
        h.advance(Duration::from_millis(16));
    }
}

#[test]
fn a_press_that_travels_lifts_the_widget_under_the_pointer() {
    let mut run = start("lift", vec![WidgetKind::Calendar, WidgetKind::Battery]);
    let h = &mut run.harness;
    assert_eq!(lifted(h, "widget-calendar"), 0);
    pick_up(h, (1100.0, 98.0));
    assert_eq!(lifted(h, "widget-calendar"), 1, "lifted once it travels");
    assert_eq!(h.count(".sill-dw-shield"), 1, "its controls are shielded");
    assert_eq!(
        box_of(h, "widget-calendar"),
        (1018, 16, 164, 164),
        "it follows the pointer 1:1"
    );
    assert!(
        !h.attr(".sill-dw-place:last-child", "style")
            .is_some_and(|s| s.contains("transform")),
        "the lift is the card's own, not the layer's"
    );
    assert_eq!(
        guide(h),
        Some((1100, 16, 164, 164)),
        "quire's slot guide at the cell it would land in"
    );
    // Held still, the lift settles: nothing moves while it is held.
    assert_settles_to_zero_frames(h);
    h.pointer_up(point(1100.0, 98.0));
    settle_until(h, |h| h.count(".ds-widget[*|data-lift=lifted]") == 0);
    assert_eq!(h.count(".sill-dw-shield"), 0);
    assert_eq!(guide(h), None, "no guide at rest");
    assert_settles_to_zero_frames(h);
}

#[test]
fn a_drop_snaps_into_the_grid_and_is_saved_for_this_output() {
    let mut run = start("drop", vec![WidgetKind::Calendar, WidgetKind::Battery]);
    let h = &mut run.harness;
    // The corner goes to (478, 218): nearest column 3 (560) and row 1 (196).
    pick_up(h, (560.0, 300.0));
    assert_eq!(box_of(h, "widget-calendar"), (478, 218, 164, 164));
    assert_eq!(guide(h), Some((560, 196, 164, 164)));
    h.pointer_up(point(560.0, 300.0));
    h.advance(Duration::from_millis(16));
    assert_ne!(
        box_of(h, "widget-calendar"),
        (560, 196, 164, 164),
        "it glides into its cell rather than jumping"
    );
    settle_until(h, |h| box_of(h, "widget-calendar") == (560, 196, 164, 164));
    assert_settles_to_zero_frames(h);
    assert_eq!(box_of(h, "widget-calendar"), (560, 196, 164, 164));
    assert_eq!(box_of(h, "widget-battery"), (1100, 196, 164, 164));
    assert_eq!(
        run.file.load(&OutputKey::of(Some("DP-1"))),
        vec![
            at(WidgetKind::Calendar, 3, 1),
            at(WidgetKind::Battery, 6, 1)
        ]
    );
}

#[test]
fn the_widget_it_is_over_makes_room_and_goes_back_after() {
    let mut run = start("room", vec![WidgetKind::Calendar, WidgetKind::Battery]);
    let h = &mut run.harness;
    // The Calendar held over the Battery's cell (1100, 196): the Battery steps up into the
    // Calendar's old cell, gliding there.
    pick_up(h, (1182.0, 278.0));
    h.advance(Duration::from_millis(16));
    assert_ne!(
        box_of(h, "widget-battery"),
        (1100, 16, 164, 164),
        "it glides"
    );
    settle_until(h, |h| box_of(h, "widget-battery") == (1100, 16, 164, 164));
    assert_settles_to_zero_frames(h);
    // Moved on to an empty cell: the Battery goes back to its own place.
    h.pointer_move(point(500.0, 600.0));
    settle_until(h, |h| box_of(h, "widget-battery") == (1100, 196, 164, 164));
    assert_settles_to_zero_frames(h);
    // Back over it and dropped there: the swap is kept.
    h.pointer_move(point(1182.0, 278.0));
    h.pointer_up(point(1182.0, 278.0));
    settle_until(h, |h| {
        box_of(h, "widget-calendar") == (1100, 196, 164, 164)
            && box_of(h, "widget-battery") == (1100, 16, 164, 164)
    });
    assert_settles_to_zero_frames(h);
    assert_eq!(
        run.file.load(&OutputKey::of(Some("DP-1"))),
        vec![
            at(WidgetKind::Calendar, 6, 1),
            at(WidgetKind::Battery, 6, 0)
        ]
    );
}

fn sent(feeds: &mut fake::Feeds) -> Vec<MediaCommand> {
    std::iter::from_fn(|| feeds.media_commands.try_recv().ok()).collect()
}

fn pressed_next(commands: &[MediaCommand]) -> usize {
    commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                MediaCommand::Press {
                    verb: MediaVerb::Next,
                    ..
                }
            )
        })
        .count()
}

#[test]
fn a_click_on_a_control_still_works_and_a_drag_from_it_does_not_press_it() {
    let mut run = start("click", vec![WidgetKind::NowPlaying]);
    run.feeds.media.send_replace(fake::media());
    run.harness.advance(Duration::from_millis(100));
    let before = box_of(&run.harness, "widget-now-playing");
    let next = run
        .harness
        .centre("#widget-now-playing .sill-cc-media-buttons .ds-icon-button:last-child")
        .expect("the Next button is laid out");
    run.harness.click(next);
    settle_until(&mut run.harness, |h| h.count(".sill-dw-shield") == 0);
    run.harness.advance(Duration::from_millis(50));
    assert_eq!(
        pressed_next(&sent(&mut run.feeds)),
        1,
        "the click pressed Next"
    );
    assert_eq!(
        box_of(&run.harness, "widget-now-playing"),
        before,
        "nothing moved"
    );
    assert_settles_to_zero_frames(&mut run.harness);

    // A press on the same button that travels drags the widget instead; its release lands on
    // the shield, never on the button that moved along under it.
    let h = &mut run.harness;
    h.pointer_move(next);
    h.pointer_down(next);
    let away = point(next.x.0 - 400.0, next.y.0 + 200.0);
    for step in 1..=4 {
        let part = step as f32 / 4.0;
        h.pointer_move(point(
            next.x.0 + (away.x.0 - next.x.0) * part,
            next.y.0 + (away.y.0 - next.y.0) * part,
        ));
        h.advance(Duration::from_millis(16));
    }
    assert_eq!(lifted(h, "widget-now-playing"), 1);
    h.pointer_up(away);
    settle_until(h, |h| h.count(".ds-widget[*|data-lift=lifted]") == 0);
    assert_settles_to_zero_frames(h);
    assert_eq!(
        pressed_next(&sent(&mut run.feeds)),
        0,
        "the drag pressed nothing"
    );
    assert_ne!(
        box_of(&run.harness, "widget-now-playing"),
        before,
        "it moved"
    );
}
