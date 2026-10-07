//! The Calendar and Up Next widgets on the fake services, in a Popover `Ds` as the notification
//! center holds them: the month with today on the accent disc and busy days dotted, the week
//! column on `calendar.week_numbers = "show"`, the previous / next buttons (the card's
//! `MonthIntent::Step`), the Medium card's today column and its next event from the agenda Up
//! Next reads (the center's month is Medium), Up Next's rows in order with their calendar
//! colour, its empty and quiet lines, and all through quire's markup lint (coherence rules 1
//! and 2).

use sill_ui_kit::harness::fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use ds_lint::{LintConfig, Profile, markup};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use sill_model::events::{
    Agenda, CalEvent, CalendarName, EventTime, EventUid, EventsError, SourceStatus,
};
use sill_settings::ShellFile;
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::widgets::{Widget, WidgetHost, WidgetKind, widgets_css};

/// Today's number, on the accent disc.
const TODAY: &str = ".ds-month-day[*|aria-current=date] .ds-month-num";

/// Which widget the harness draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Calendar,
    CalendarOnDesktop,
    UpNext,
}

#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    kind: Kind,
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
    let css = widgets_css();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Popover,
            AppStyle { css: css }
            match world.kind {
                Kind::Calendar => rsx! { {widget(WidgetKind::Calendar, WidgetHost::Tile)} },
                Kind::CalendarOnDesktop => rsx! { {widget(WidgetKind::Calendar, WidgetHost::Desktop)} },
                Kind::UpNext => rsx! { {widget(WidgetKind::UpNext, WidgetHost::Tile)} },
            }
        }
    }
}

/// `kind` in `host` at its one size.
fn widget(kind: WidgetKind, host: WidgetHost) -> Element {
    rsx! { Widget { kind, host } }
}

/// A widget drawn and what feeds it.
struct Running {
    harness: Harness,
    _feeds: fake::Feeds,
}

fn start(kind: Kind, file: ShellFile, agenda: Agenda) -> Running {
    let (services, feeds) = fake::services();
    feeds.agenda.send_replace(agenda);
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            kind,
        });
    });
    let mut harness = Harness::new(
        app,
        HarnessConfig::new(Viewport {
            width: 380,
            height: 420,
            scale_percent: 100,
        })
        .with_clock(Clock::Virtual),
    );
    harness.advance(Duration::from_millis(60));
    Running {
        harness,
        _feeds: feeds,
    }
}

fn zone() -> TimeZone {
    TimeZone::try_system().unwrap_or(TimeZone::UTC)
}

fn now() -> Zoned {
    Zoned::now().with_time_zone(zone())
}

fn event(uid: &str, calendar: &str, when: EventTime) -> CalEvent {
    CalEvent {
        uid: EventUid(uid.to_owned()),
        title: uid.to_owned(),
        when,
        location: None,
        calendar: CalendarName(calendar.to_owned()),
    }
}

fn all_day(uid: &str, day: Date) -> CalEvent {
    event(
        uid,
        "home",
        EventTime::AllDay {
            first: day,
            last: day,
        },
    )
}

fn timed(uid: &str, calendar: &str, start: Timestamp, minutes: i64) -> CalEvent {
    let end = start.checked_add(minutes.minutes()).unwrap_or(start);
    event(uid, calendar, EventTime::Timed { start, end })
}

fn read(events: Vec<CalEvent>) -> Agenda {
    Agenda {
        today: now().date(),
        events,
        status: SourceStatus::Read {
            calendars: 2,
            problems: Vec::new(),
        },
    }
}

fn clean(html: &str) {
    let css = format!("{}\n{}", sill_shell_kit::stylesheet(), widgets_css());
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&sill_shell_kit::kits())
    };
    let offences = markup(html, &css, &strict);
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_month_marks_today_and_the_busy_days() {
    let today = now().date();
    let running = start(
        Kind::Calendar,
        ShellFile::default(),
        read(vec![all_day("a", today), all_day("b", today)]),
    );
    let h = &running.harness;
    assert_eq!(h.count(".ds-month-head"), 7);
    let rows = h.count(".ds-month-row") - 1;
    assert!((4..=6).contains(&rows), "{rows} weeks");
    assert_eq!(h.count(".ds-month-day"), rows * 7);
    assert_eq!(h.count(TODAY), 1);
    assert_eq!(
        h.text_of(TODAY).as_deref(),
        Some(today.day().to_string().as_str())
    );
    assert_eq!(
        h.count(".ds-month-dot"),
        1,
        "one busy day, however many events"
    );
    assert_eq!(
        h.count(".ds-month-week"),
        0,
        "week numbers are hidden by default"
    );
    let disc = h.rect(TODAY).expect("today is laid out");
    assert!(
        (disc.size.width.0 - disc.size.height.0).abs() < 0.5 && disc.size.width.0 > 10.0,
        "a disc: {disc:?}"
    );
    clean(&h.html());
}

/// The center's month is Medium (its one size there): today and the day's next event beside the
/// compact month, filling the tile.
#[test]
fn the_centers_month_has_today_and_the_next_event_from_the_agenda_up_next_reads() {
    let at = now().timestamp();
    let later = |m: i64| at.checked_add(m.minutes()).unwrap_or(at);
    let today = now().date();
    let events = vec![
        all_day("holiday", today),
        timed("standup", "work", later(1), 1),
        timed("review", "home", later(2), 1),
    ];
    let running = start(Kind::Calendar, ShellFile::default(), read(events));
    let h = &running.harness;
    assert_eq!(
        h.attr("#widget-calendar", "data-size").as_deref(),
        Some("medium")
    );
    assert_eq!(
        h.text_of(".ds-month-today .ds-month-event-title")
            .as_deref(),
        Some("holiday"),
        "the all-day event first, as Up Next orders them"
    );
    assert_eq!(
        h.attr(".ds-month", "data-density").as_deref(),
        Some("compact")
    );
    let card = h.rect("#widget-calendar").expect("the card");
    let month = h.rect(".ds-month").expect("the month");
    assert!(
        month.origin.y.0 + month.size.height.0 >= card.origin.y.0 + card.size.height.0 - 40.0,
        "the month reaches the card's foot, not its top two thirds: {card:?} {month:?}"
    );
    clean(&h.html());
    let quiet = start(Kind::Calendar, ShellFile::default(), read(Vec::new()));
    assert_eq!(
        quiet.harness.text_of(".ds-month-quiet").as_deref(),
        Some("No events today")
    );
}

#[test]
fn up_next_lists_the_next_events_in_order_with_their_colour() {
    let at = now().timestamp();
    let hour = |h: i64| at.checked_add(h.hours()).unwrap_or(at);
    let events = vec![
        timed("later", "work", hour(3), 30),
        timed("soon", "home", hour(1), 30),
        timed("over", "home", hour(-3), 30),
        timed("far", "home", hour(24 * 20), 30),
    ];
    let running = start(Kind::UpNext, ShellFile::default(), read(events));
    let h = &running.harness;
    assert_eq!(h.count(".sill-un-row"), 2);
    assert_eq!(
        h.text_of(".sill-un-row:first-child .sill-un-title")
            .as_deref(),
        Some("soon")
    );
    assert_eq!(
        h.count(".sill-un-bar"),
        2,
        "each row has its calendar's colour"
    );
    assert_eq!(h.count(".sill-un-empty"), 0);
    assert_eq!(h.count(".sill-un-quiet"), 0);
    clean(&h.html());
}

#[test]
fn up_next_says_when_there_is_nothing_and_why() {
    let empty = start(Kind::UpNext, ShellFile::default(), read(Vec::new()));
    assert_eq!(
        empty.harness.text_of(".sill-un-empty").as_deref(),
        Some("No upcoming events")
    );
    clean(&empty.harness.html());

    let off = start(
        Kind::UpNext,
        ShellFile::default(),
        Agenda::starting(now().date()),
    );
    assert_eq!(
        off.harness.text_of(".sill-un-quiet").as_deref(),
        Some("Calendars are off")
    );
    assert_eq!(off.harness.count(".sill-un-empty"), 0);

    let mut broken = read(Vec::new());
    broken.status = SourceStatus::Read {
        calendars: 1,
        problems: vec![EventsError::Unreadable {
            path: "/c/std.ics".into(),
            reason: "gone".to_owned(),
        }],
    };
    let broken = start(Kind::UpNext, ShellFile::default(), broken);
    assert_eq!(
        broken.harness.text_of(".sill-un-quiet").as_deref(),
        Some("1 calendar file could not be read")
    );
}

#[test]
fn on_the_desktop_the_calendar_is_the_compact_month_inside_its_small_frame() {
    let mut running = start(
        Kind::CalendarOnDesktop,
        ShellFile::default(),
        read(Vec::new()),
    );
    let h = &running.harness;
    assert_eq!(
        h.attr(".ds-month", "data-density").as_deref(),
        Some("compact"),
        "the small frame asks quire's month to be compact (Q190)"
    );
    assert_eq!(
        h.count(".ds-month [*|aria-current=date]"),
        1,
        "today marked"
    );
    clean(&h.html());
    // At the default 164 px cell every month stays inside the frame's content box (quire's
    // `.ds-widget-body`, measured rather than assumed, since quire sets the padding), a six-week
    // one included (twelve months hold at least one).
    let mut weeks = Vec::new();
    for _ in 0..12 {
        let h = &running.harness;
        let frame = h.rect("#widget-calendar").expect("the frame is laid out");
        let body = h
            .rect("#widget-calendar .ds-widget-body")
            .expect("the frame's content box is laid out");
        let month = h.rect(".ds-month").expect("the month is laid out");
        assert_eq!(frame.size.width.0.round(), 164.0);
        let x = within(
            month.origin.x.0,
            month.size.width.0,
            body.origin.x.0,
            body.size.width.0,
        );
        let y = within(
            month.origin.y.0,
            month.size.height.0,
            body.origin.y.0,
            body.size.height.0,
        );
        assert!(
            x && y,
            "the month {month:?} inside the frame's content box {body:?}"
        );
        weeks.push(h.count(".ds-month-weeks .ds-month-row"));
        let next = h
            .centre(".ds-month-step:last-child")
            .expect("the next button");
        running.harness.send(Input::click(next));
        running.harness.advance(Duration::from_millis(600));
    }
    assert!(weeks.contains(&6), "a six-week month was drawn: {weeks:?}");
}

/// Whether a span starting at `from`, `size` long, lies inside a frame's 16 px padding.
fn within(from: f32, size: f32, box_from: f32, box_size: f32) -> bool {
    from >= box_from - 0.5 && from + size <= box_from + box_size + 0.5
}
