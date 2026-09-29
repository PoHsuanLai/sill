use jiff::civil::date;
use jiff::tz::TimeZone;

use super::{Exclusions, Shape, overlaps, resolve, zone_named};
use crate::events::{EventTime, IcsEnd, IcsEvent, IcsStatus, IcsTime, Window};

fn taipei() -> TimeZone {
    TimeZone::get("Asia/Taipei").unwrap_or_else(|e| panic!("{e}"))
}

fn event(start: IcsTime, end: IcsEnd) -> IcsEvent {
    IcsEvent {
        uid: "x".to_owned(),
        summary: "x".to_owned(),
        location: None,
        start,
        end,
        rule: None,
        exdates: Vec::new(),
        recurrence_id: None,
        status: IcsStatus::Confirmed,
    }
}

fn utc(text: &str) -> jiff::Timestamp {
    text.parse().unwrap_or_else(|e| panic!("{text}: {e}"))
}

#[test]
fn shapes_put_each_kind_of_start_on_a_day() {
    let day = date(2026, 9, 28);
    let at = date(2026, 9, 27).at(9, 0, 0, 0);
    let cases = [
        (
            event(IcsTime::Date(date(2026, 9, 27)), IcsEnd::Unsaid),
            EventTime::AllDay {
                first: day,
                last: day,
            },
        ),
        (
            event(
                IcsTime::Date(date(2026, 9, 27)),
                IcsEnd::At(IcsTime::Date(date(2026, 9, 30))),
            ),
            EventTime::AllDay {
                first: day,
                last: date(2026, 9, 30),
            },
        ),
        (
            event(IcsTime::Date(date(2026, 9, 27)), IcsEnd::Lasts(86_400 * 2)),
            EventTime::AllDay {
                first: day,
                last: date(2026, 9, 29),
            },
        ),
        (
            event(IcsTime::Floating(at), IcsEnd::Lasts(1800)),
            EventTime::Timed {
                start: utc("2026-09-28T01:00:00Z"),
                end: utc("2026-09-28T01:30:00Z"),
            },
        ),
        (
            event(
                IcsTime::Zoned {
                    at,
                    zone: "Europe/Berlin".to_owned(),
                },
                IcsEnd::At(IcsTime::Zoned {
                    at: date(2026, 9, 27).at(10, 0, 0, 0),
                    zone: "Europe/Berlin".to_owned(),
                }),
            ),
            EventTime::Timed {
                start: utc("2026-09-28T07:00:00Z"),
                end: utc("2026-09-28T08:00:00Z"),
            },
        ),
        (
            event(IcsTime::Utc(utc("2026-09-27T23:30:00Z")), IcsEnd::Unsaid),
            EventTime::Timed {
                start: utc("2026-09-28T23:30:00Z"),
                end: utc("2026-09-28T23:30:00Z"),
            },
        ),
    ];
    for (event, want) in cases {
        let shape = Shape::of(&event, &taipei());
        assert_eq!(shape.at(day), Some(want), "{event:?}");
    }
}

#[test]
fn a_time_skipped_by_a_clock_change_moves_forward() {
    let event = event(
        IcsTime::Zoned {
            at: date(2026, 3, 1).at(2, 30, 0, 0),
            zone: "America/New_York".to_owned(),
        },
        IcsEnd::Unsaid,
    );
    let shape = Shape::of(&event, &taipei());
    assert_eq!(
        shape.at(date(2026, 3, 8)),
        Some(EventTime::Timed {
            start: utc("2026-03-08T07:30:00Z"),
            end: utc("2026-03-08T07:30:00Z"),
        })
    );
}

#[test]
fn zones_resolve_by_name_prefix_or_fall_back() {
    let viewer = taipei();
    let name = |zone: &TimeZone| zone.iana_name().map(str::to_owned);
    let cases = [
        ("Europe/Berlin", Some("Europe/Berlin")),
        (
            "/mozilla.org/20050126_1/Europe/Berlin",
            Some("Europe/Berlin"),
        ),
        ("Taipei Standard Time", Some("Asia/Taipei")),
    ];
    for (text, want) in cases {
        assert_eq!(name(&zone_named(text, &viewer)).as_deref(), want, "{text}");
    }
}

#[test]
fn exclusions_match_a_day_or_an_instant() {
    let zone = taipei();
    let skip = Exclusions::of(
        &[
            IcsTime::Date(date(2026, 9, 28)),
            IcsTime::Zoned {
                at: date(2026, 9, 30).at(9, 0, 0, 0),
                zone: "Asia/Taipei".to_owned(),
            },
        ],
        &zone,
    );
    let timed = |text| EventTime::Timed {
        start: utc(text),
        end: utc(text),
    };
    assert!(skip.covers(&timed("2026-09-28T01:00:00Z"), &zone));
    assert!(skip.covers(&timed("2026-09-30T01:00:00Z"), &zone));
    assert!(!skip.covers(&timed("2026-09-29T01:00:00Z"), &zone));
    let day = |d| EventTime::AllDay {
        first: date(2026, 9, d),
        last: date(2026, 9, d),
    };
    assert!(skip.covers(&day(28), &zone));
    assert!(skip.covers(&day(30), &zone));
    assert!(!skip.covers(&day(29), &zone));
}

#[test]
fn overlap_reads_the_window_in_the_viewers_days() {
    let zone = taipei();
    let window = Window {
        from: date(2026, 9, 28),
        until: date(2026, 9, 29),
    };
    let timed = |from: &str, to: &str| EventTime::Timed {
        start: utc(from),
        end: utc(to),
    };
    let cases = [
        (timed("2026-09-27T15:00:00Z", "2026-09-27T16:00:00Z"), false),
        (timed("2026-09-27T14:00:00Z", "2026-09-27T16:30:00Z"), true),
        (timed("2026-09-27T16:00:00Z", "2026-09-27T16:00:00Z"), true),
        (timed("2026-09-27T15:00:00Z", "2026-09-27T15:00:00Z"), false),
        (timed("2026-09-28T15:00:00Z", "2026-09-28T17:00:00Z"), true),
        (timed("2026-09-28T16:00:00Z", "2026-09-28T17:00:00Z"), false),
        (
            EventTime::AllDay {
                first: date(2026, 9, 27),
                last: date(2026, 9, 28),
            },
            true,
        ),
        (
            EventTime::AllDay {
                first: date(2026, 9, 29),
                last: date(2026, 9, 29),
            },
            false,
        ),
    ];
    for (when, want) in cases {
        assert_eq!(overlaps(&when, window, &zone), want, "{when:?}");
    }
    assert_eq!(
        resolve(&IcsTime::Date(date(2026, 9, 28)), &zone),
        Some(utc("2026-09-27T16:00:00Z"))
    );
}
