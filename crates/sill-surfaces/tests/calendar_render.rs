//! The Calendar and Up Next widgets on the fake services, in a Popover `Ds` as the notification
//! center holds them: the month with today on the accent disc and busy days dotted, the week
//! column on `calendar.week_numbers = "show"`, the previous / next buttons, Up Next's rows in
//! order with their calendar colour, its empty and quiet lines, and both through quire's markup
//! lint (coherence rules 1 and 2).

#[path = "support/fake.rs"]
mod fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::lint::{LintConfig, Profile, markup};
use ds::{Appearance, Ds, Material};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use sill_services::events::{CalendarName, EventUid, EventsError};
use sill_services::{Agenda, CalEvent, EventTime, Services, SourceStatus};
use sill_settings::{SettingsFeed, ShellFile, WeekNumbers};
use sill_surfaces::widgets::calendar::CalendarWidget;
use sill_surfaces::widgets::up_next::UpNextWidget;
use sill_surfaces::widgets::{Widget, WidgetHost, WidgetKind, widgets_css};

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
    use_context_provider(|| world.settings.clone());
    let css = widgets_css();
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover,
            style { {css} }
            match world.kind {
                Kind::Calendar => rsx! { CalendarWidget { host: WidgetHost::Tile } },
                Kind::CalendarOnDesktop => rsx! { Widget { kind: WidgetKind::Calendar, host: WidgetHost::Desktop } },
                Kind::UpNext => rsx! { UpNextWidget { host: WidgetHost::Tile } },
            }
        }
    }
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
    let mut harness = Harness::with_config(
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
    let css = format!("{}\n{}", ds::stylesheet(), widgets_css());
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
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

#[test]
fn week_numbers_add_a_column() {
    let mut file = ShellFile::default();
    file.calendar.week_numbers = WeekNumbers::Show;
    let running = start(Kind::Calendar, file, read(Vec::new()));
    let h = &running.harness;
    let rows = h.count(".ds-month-row");
    assert_eq!(
        h.count(".ds-month-week"),
        rows,
        "a head cell and one per week"
    );
    clean(&h.html());
}

#[test]
fn next_and_previous_step_the_month() {
    let mut running = start(Kind::Calendar, ShellFile::default(), read(Vec::new()));
    let title = |h: &Harness| h.text_of(".ds-month-title").unwrap_or_default();
    let first = title(&running.harness);
    let click = |running: &mut Running, index: usize| {
        let selector = format!(".ds-month-header .ds-icon-button:nth-child({index})");
        let rect = running.harness.rect(&selector).expect("the step button");
        running.harness.click(ds::Point {
            x: ds::Px(rect.origin.x.0 + rect.size.width.0 / 2.0),
            y: ds::Px(rect.origin.y.0 + rect.size.height.0 / 2.0),
        });
        running.harness.advance(Duration::from_millis(60));
    };
    click(&mut running, 3);
    let next = title(&running.harness);
    assert_ne!(next, first, "next month");
    click(&mut running, 2);
    assert_eq!(title(&running.harness), first, "and back");
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
        running.harness.click(next);
        running.harness.advance(Duration::from_millis(600));
    }
    assert!(weeks.contains(&6), "a six-week month was drawn: {weeks:?}");
}

/// Whether a span starting at `from`, `size` long, lies inside a frame's 16 px padding.
fn within(from: f32, size: f32, box_from: f32, box_size: f32) -> bool {
    from >= box_from - 0.5 && from + size <= box_from + box_size + 0.5
}
