//! What the widgets draw, each in a `ds_native::Harness`
//! on the fake services: the card (quire's `WidgetCard`: a tile in the notification center, the
//! Widget material's card on the desktop, each at its footprint, `data-widget` its kind),
//! Battery's rings from the machine and the connected devices and their bump on a change, Now
//! Playing as the control center's view (its presses reach the fake media queue, never a real
//! player), World Clock's faces from `widgets.world_clocks`; and the notification center's
//! slot, which draws `widgets.center` in order under the history, each at its one size
//! (Medium), and "Edit Widgets" under it.

use center_support::fake;
use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Ds, Material, RootExtent};
use ds_native::harness::settle_until;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::now_playing::{MediaCommand, MediaVerb};
use sill_settings::{ShellFile, WidgetKind};
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::widgets::{Widget, WidgetHost, widgets_css};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    kind: WidgetKind,
    host: WidgetHost,
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
    let material = match world.host {
        WidgetHost::Tile => Material::Popover,
        WidgetHost::Desktop => Material::Widget,
    };
    rsx! {
        Ds { appearance: ds::Appearance::default(), material, extent: RootExtent::Viewport,
            style { {widgets_css()} }
            div { style: "width:344px;",
                Widget { kind: world.kind, host: world.host }
            }
        }
    }
}

const VIEWPORT: Viewport = Viewport {
    width: 480,
    height: 480,
    scale_percent: 100,
};

/// A running widget and its feeds.
struct Running {
    harness: Harness,
    feeds: fake::Feeds,
}

fn start(kind: WidgetKind, host: WidgetHost, file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    start_with(services, feeds, kind, host, file)
}

/// `start` on services already fed (a player playing before the widget mounts).
fn start_with(
    services: Services,
    feeds: fake::Feeds,
    kind: WidgetKind,
    host: WidgetHost,
    file: ShellFile,
) -> Running {
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            kind,
            host,
        });
    });
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(100));
    Running { harness, feeds }
}

#[test]
fn in_the_center_the_frame_is_quires_tile_with_no_header_row() {
    let running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    let h = &running.harness;
    assert_eq!(h.count(".ds-widget#widget-battery"), 1);
    assert_eq!(
        h.attr("#widget-battery", "data-host").as_deref(),
        Some("tile")
    );
    assert_eq!(
        h.attr("#widget-battery", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.count("#widget-battery .ds-widget-title"),
        0,
        "Battery is a glanceable widget with no header (design/23-WIDGETS.md M25)"
    );
    assert_eq!(
        h.count(".ds[*|data-material=widget]"),
        0,
        "no second material"
    );
    let tile = h.rect("#widget-battery").expect("the tile is laid out");
    assert_eq!(
        (tile.size.width.0.round(), tile.size.height.0.round()),
        (344.0, 164.0)
    );
}

#[test]
fn on_the_desktop_the_frame_is_a_surface_in_the_widget_material() {
    let running = start(
        WidgetKind::Battery,
        WidgetHost::Desktop,
        ShellFile::default(),
    );
    let h = &running.harness;
    assert_eq!(
        h.attr("#widget-battery", "data-host").as_deref(),
        Some("desktop")
    );
    assert_eq!(
        h.attr("#widget-battery", "data-size").as_deref(),
        Some("small")
    );
    assert_eq!(
        h.count(".ds[*|data-material=widget] > .ds-widget#widget-battery"),
        1,
        "the card is the Widget material's plate: {}",
        h.html()
    );
    let card = h.rect("#widget-battery").expect("the card is laid out");
    assert_eq!(
        (card.size.width.0.round(), card.size.height.0.round()),
        (164.0, 164.0)
    );
}

#[test]
fn battery_lists_the_machine_then_connected_devices_with_a_level() {
    let mut running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    // quire's ring and figure sweep from empty on mount (design/23-WIDGETS.md section 4.1); let
    // that first fill settle before reading the figures' final count.
    settle_until(&mut running.harness, |h| {
        h.count("#widget-battery .ds-battery[*|data-pulse=a]") == 0
    });
    let h = &running.harness;
    // The fake machine is at 64%; Buds are connected at 80%; the keyboard is not connected. The
    // Medium row has four fixed places: the two batteries from the left, then two bare
    // places with no number.
    assert_eq!(h.count("#widget-battery .ds-batteries-cell"), 4);
    assert_eq!(
        h.count("#widget-battery .ds-batteries-cell[*|data-place=empty]"),
        2
    );
    assert_eq!(
        h.text_of("#widget-battery .ds-batteries-cell:nth-child(3) .ds-batteries-figure")
            .as_deref(),
        Some(""),
        "a place with no battery shows no number"
    );
    let cell = |n: usize, part: &str| {
        h.attr(
            &format!("#widget-battery .ds-batteries-cell:nth-child({n}) .ds-battery"),
            part,
        )
    };
    assert_eq!(
        (cell(1, "aria-valuenow"), cell(2, "aria-valuenow")),
        (Some("64".to_owned()), Some("80".to_owned()))
    );
    let figure = |n: usize| {
        h.text_of(&format!(
            "#widget-battery .ds-batteries-cell:nth-child({n}) .ds-battery-figure"
        ))
    };
    assert_eq!(
        (figure(1), figure(2)),
        (Some("64%".to_owned()), Some("80%".to_owned())),
        "each row's figure settles on its true percentage"
    );
    let html = h.html();
    assert!(html.contains("Buds"), "the headphones");
    assert!(
        !html.contains("Keychron"),
        "a disconnected device is left out"
    );
}

#[test]
fn battery_says_so_when_there_is_none() {
    let mut running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    running
        .feeds
        .battery
        .send_replace(sill_model::battery::Battery::default());
    running
        .feeds
        .bluetooth
        .send_replace(sill_model::bluetooth::Bluetooth::default());
    running.harness.advance(Duration::from_millis(100));
    let h = &running.harness;
    assert_eq!(h.count("#widget-battery .ds-battery"), 0);
    assert_eq!(
        h.text_of("#widget-battery .ds-batteries-quiet").as_deref(),
        Some("No batteries")
    );
}

#[test]
fn now_playing_is_the_control_centers_view_and_presses_reach_the_fake() {
    let mut running = start(
        WidgetKind::NowPlaying,
        WidgetHost::Tile,
        ShellFile::default(),
    );
    running.feeds.media.send_replace(fake::media());
    running.harness.advance(Duration::from_millis(100));
    let h = &running.harness;
    assert_eq!(
        h.text_of("#widget-now-playing .ds-track-title").as_deref(),
        Some("So What")
    );
    assert_eq!(
        h.count("#widget-now-playing .ds-module-panel"),
        0,
        "the card is the plate: no module panel inside it"
    );
    assert_eq!(
        h.attr("#widget-now-playing", "data-widget").as_deref(),
        Some("sill.now-playing")
    );
    let next = h
        .centre("#widget-now-playing .sill-cc-media-buttons .ds-icon-button:last-child")
        .expect("the Next button is laid out");
    running.harness.click(next);
    running.harness.advance(Duration::from_millis(50));
    let sent: Vec<_> =
        std::iter::from_fn(|| running.feeds.media_commands.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|c| matches!(
            c,
            MediaCommand::Press {
                verb: MediaVerb::Next,
                ..
            }
        )),
        "{sent:?}"
    );
}

/// The live center mounts Now Playing with a player already there (a `sill reload` that adds
/// it): the card shows the track from its first entry.
#[test]
fn now_playing_mounted_over_a_playing_player_shows_its_track() {
    let (services, feeds) = fake::services();
    feeds.media.send_replace(fake::media());
    let running = start_with(
        services,
        feeds,
        WidgetKind::NowPlaying,
        WidgetHost::Tile,
        ShellFile::default(),
    );
    assert_eq!(
        running
            .harness
            .text_of("#widget-now-playing .ds-track-title")
            .as_deref(),
        Some("So What"),
        "{}",
        running.harness.html()
    );
}

#[test]
fn world_clock_draws_a_row_per_known_city() {
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London; Nowhere/Atlantis; America/New_York".to_owned();
    let running = start(WidgetKind::WorldClock, WidgetHost::Tile, file);
    let h = &running.harness;
    assert_eq!(
        h.count("#widget-world-clock .ds-clock[*|data-look=digital]"),
        2
    );
    let html = h.html();
    assert!(
        html.contains("London") && html.contains("New York"),
        "{html}"
    );
    assert!(!html.contains("Atlantis"));
}

#[test]
fn world_clock_with_no_city_names_the_key() {
    let running = start(
        WidgetKind::WorldClock,
        WidgetHost::Tile,
        ShellFile::default(),
    );
    let text = running
        .harness
        .text_of("#widget-world-clock .ds-world-clock-quiet")
        .unwrap_or_default();
    assert!(text.contains("widgets.world_clocks"), "{text}");
}

#[test]
fn the_center_draws_widgets_center_in_order_under_the_history() {
    let running = center_support::opened(center_support::two_apps());
    let h = &running.harness;
    assert_eq!(h.count(".sill-nc-widgets > .ds-widget"), 3);
    let ids: Vec<_> = (1..=3)
        .map(|n| {
            h.attr(
                &format!(".sill-nc-widgets > .ds-widget:nth-child({n})"),
                "id",
            )
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            Some("widget-calendar".to_owned()),
            Some("widget-up-next".to_owned()),
            Some("widget-battery".to_owned()),
        ]
    );
    let list = h.rect(".sill-nc-list").expect("the history");
    let first = h.rect("#widget-calendar").expect("the first widget");
    assert!(
        first.origin.y.0 >= list.origin.y.0 + list.size.height.0 - 0.5,
        "under the history: {list:?} {first:?}"
    );
    // A medium tile (344) fits the center's 384 px column after the panel's padding.
    let column = h.rect(".sill-nc-column").expect("the column");
    let tile = h.rect("#widget-battery").expect("a medium tile");
    assert_eq!(tile.size.width.0.round(), 344.0);
    assert!(
        tile.origin.x.0 >= column.origin.x.0 - 0.5
            && tile.origin.x.0 + tile.size.width.0 <= column.origin.x.0 + column.size.width.0 + 0.5,
        "inside the column: {column:?} {tile:?}"
    );
}

#[test]
fn on_the_desktop_the_world_clock_draws_dials_tinted_for_day_or_night() {
    let mut file = ShellFile::default();
    file.widgets.world_clocks = "Europe/London; Asia/Tokyo".to_owned();
    let running = start(WidgetKind::WorldClock, WidgetHost::Desktop, file);
    let h = &running.harness;
    assert_eq!(
        h.attr("#widget-world-clock", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.count("#widget-world-clock .ds-clock[*|data-look=analog]"),
        2
    );
    assert_eq!(h.count("#widget-world-clock .ds-clock-dial"), 2);
    // Nine hours apart: at most one of the two can be between 06:00 and 18:00 while the other is
    // not, and each face carries a phase.
    assert_eq!(h.count("#widget-world-clock .ds-clock[*|data-phase]"), 2);
}

/// Renamed from `a_battery_level_that_changes_bumps_once`: sill no longer wears its own
/// `Bumped` pulse on the percentage text (quire's `BatteryFigure` draws it now), so
/// `.ds-bumped.a-bump` never appears; and "nothing bumps on mount" no longer holds; quire's ring
/// always sweeps from empty on mount (design/23-WIDGETS.md section 4.1), so a fresh widget is
/// moving from its first frame. What still holds, and what this checks instead: a level change
/// bumps the ring's own `data-pulse` to `a` while it re-sweeps, then it comes to rest.
#[test]
fn a_battery_level_that_changes_bumps_the_ring_again() {
    let mut running = start(WidgetKind::Battery, WidgetHost::Tile, ShellFile::default());
    let moving = "#widget-battery .ds-battery[*|data-pulse=a]";
    settle_until(&mut running.harness, |h| h.count(moving) == 0);
    let mut battery = running.feeds.battery.borrow().clone();
    battery.level = sill_model::units::Percent(65);
    running.feeds.battery.send_replace(battery);
    running.harness.advance(Duration::from_millis(30));
    assert!(
        running.harness.count(moving) >= 1,
        "the new level bumps: {}",
        running.harness.html()
    );
    settle_until(&mut running.harness, |h| h.count(moving) == 0);
    assert_eq!(running.harness.count(moving), 0, "and comes to rest");
}

#[test]
fn opening_the_center_again_replays_the_batterys_fill() {
    let mut running = center_support::opened(center_support::two_apps());
    let moving = "#widget-battery .ds-battery[*|data-pulse=a]";
    // Let the first opening's mount-sweep settle, so the next `moving` we see is the reopening's
    // own wake, not a straggler from mounting.
    settle_until(&mut running.harness, |h| h.count(moving) == 0);
    running.link.close();
    center_support::settle(&mut running.harness, |h| h.count(".ds-panel") == 0);
    running.link.open(center_support::fake::output().id);
    center_support::settle(&mut running.harness, |h| h.count(".ds-panel") == 1);
    assert!(
        running.harness.count(moving) >= 1,
        "the widget host's wake replays the ring's fill on reopen: {}",
        running.harness.html()
    );
    settle_until(&mut running.harness, |h| h.count(moving) == 0);
}

#[test]
fn the_card_names_its_kind() {
    let cases = [
        (WidgetKind::Calendar, "quire.month"),
        (WidgetKind::UpNext, "sill.up-next"),
        (WidgetKind::Battery, "quire.battery"),
        (WidgetKind::NowPlaying, "sill.now-playing"),
        (WidgetKind::WorldClock, "quire.world-clock"),
    ];
    for (kind, name) in cases {
        let running = start(kind, WidgetHost::Desktop, ShellFile::default());
        let h = &running.harness;
        assert_eq!(h.attr(".ds-widget", "data-widget").as_deref(), Some(name));
        assert_eq!(
            h.attr(".ds-widget", "data-tint").as_deref(),
            Some("space"),
            "a desktop card wears the Space tint"
        );
    }
}

/// `up_next_max` (default 3) asks for more events than a Medium tile's body has room for at the
/// default 164 px cell. With more qualifying events than fit, no row's rect may reach past the
/// card's own body rect, in either host (two rows at Medium).
#[test]
fn up_next_never_clips_a_row_past_the_tiles_body() {
    let file = ShellFile::default();
    assert_eq!(
        file.calendar.up_next_max.0, 3,
        "the default this test relies on"
    );
    let want = 2;
    for host in [WidgetHost::Tile, WidgetHost::Desktop] {
        let mut running = start(WidgetKind::UpNext, host, file.clone());
        let at = jiff::Timestamp::now();
        let hour = |h: i64| at.checked_add(jiff::ToSpan::hours(h)).unwrap_or(at);
        let event = |uid: &str, hours_ahead: i64| sill_model::events::CalEvent {
            uid: sill_model::events::EventUid(uid.to_owned()),
            title: uid.to_owned(),
            when: sill_model::events::EventTime::Timed {
                start: hour(hours_ahead),
                end: hour(hours_ahead),
            },
            location: None,
            calendar: sill_model::events::CalendarName("home".to_owned()),
        };
        let events = vec![
            event("one", 1),
            event("two", 2),
            event("three", 3),
            event("four", 4),
            event("five", 5),
            event("six", 6),
            event("seven", 7),
        ];
        running
            .feeds
            .agenda
            .send_replace(sill_model::events::Agenda {
                today: at.to_zoned(jiff::tz::TimeZone::UTC).date(),
                events,
                status: sill_model::events::SourceStatus::Read {
                    calendars: 1,
                    problems: Vec::new(),
                },
            });
        running.harness.advance(Duration::from_millis(60));
        let h = &running.harness;
        let body = h
            .rect("#widget-up-next .ds-widget-body")
            .expect("the frame's body is laid out");
        let bottom = body.origin.y.0 + body.size.height.0;
        let row_count = h.count("#widget-up-next .sill-un-row");
        assert_eq!(row_count, want, "{host:?} holds {want} whole rows");
        for n in 1..=row_count {
            let row = h
                .rect(&format!("#widget-up-next .sill-un-row:nth-child({n})"))
                .unwrap_or_else(|| panic!("row {n} is laid out in {host:?}"));
            assert!(
                row.origin.y.0 + row.size.height.0 <= bottom + 0.5,
                "row {n} clips the tile's body in {host:?}: {row:?} body_bottom={bottom}"
            );
        }
    }
}

/// The notification center opened over a history, for the widgets it draws under it.
mod center_support {
    use std::time::Duration;

    use ds_native::Harness;
    use ds_native::harness::settle_until;
    use sill_model::notifications::{
        AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification,
        NotificationId, UnixMs,
    };
    use sill_ui_kit::NotificationCenter;
    use sill_ui_kit::harness::SurfaceHarness;
    use sill_ui_kit::links::CenterLink;

    pub use sill_ui_kit::harness::fake;

    fn note(id: u32, app: &str, summary: &str) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(1_000 * i64::from(id)),
            app: AppRef {
                name: app.to_owned(),
                icon: IconRef::None,
                desktop_entry: None,
            },
            summary: summary.to_owned(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints::default(),
            expiry: Expiry::Default,
        }
    }

    /// Three from Mail (ids 5, 3, 1, newest first) and one from Chat (4).
    pub fn two_apps() -> History {
        History {
            groups: vec![
                AppGroup {
                    key: AppKey("mail".to_owned()),
                    notes: vec![
                        note(5, "Mail", "Lunch?"),
                        note(3, "Mail", "Re: plans"),
                        note(1, "Mail", "Hello"),
                    ],
                },
                AppGroup {
                    key: AppKey("chat".to_owned()),
                    notes: vec![note(4, "Chat", "ping")],
                },
            ],
        }
    }

    /// Advance until `done` holds, then one more frame's worth.
    pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
        settle_until(harness, done);
        harness.advance(Duration::from_millis(20));
    }

    /// The center opened over `history`, once its entrance has played.
    pub fn opened(history: History) -> SurfaceHarness<NotificationCenter> {
        let mut running = SurfaceHarness::<NotificationCenter>::setup(CenterLink::new(), ())
            .viewport(ds_native::Viewport {
                width: 1280,
                height: 800,
                scale_percent: 100,
            })
            .start();
        running
            .feeds
            .notifications
            .send_modify(|n| n.history = history);
        running.harness.advance(Duration::from_millis(40));
        running.link.open(fake::output().id);
        settle(&mut running.harness, |h| h.count(".ds-panel") == 1);
        running.harness.advance(Duration::from_millis(600));
        running
    }
}
