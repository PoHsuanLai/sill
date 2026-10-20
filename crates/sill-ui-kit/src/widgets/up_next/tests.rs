use jiff::civil::date;
use jiff::tz::TimeZone;
use sill_model::clock::ClockHours;
use sill_model::events::{CalEvent, EventTime};
use sill_model::events::{CalendarName, EventUid};

use super::{EventLines, calendar_hue, event_lines};
use ds::style::tokens::label_hue::LabelHue;

#[test]
fn a_calendar_keeps_one_label_hue_by_its_name() {
    // FNV-1a of the name, modulo quire's five hues (`dev/accept/calendar.sh` computes the same).
    let cases = [
        ("work", LabelHue::Violet),
        ("team", LabelHue::Green),
        ("personal", LabelHue::Amber),
    ];
    for (name, want) in cases {
        assert_eq!(calendar_hue(&CalendarName(name.to_owned())), want, "{name}");
    }
}

#[test]
fn an_events_time_line_reads_from_now() {
    let utc = TimeZone::UTC;
    let at = |d, h, m| {
        date(2026, 9, d)
            .at(h, m, 0, 0)
            .to_zoned(utc.clone())
            .map(|z| z.timestamp())
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let event = |title: &str, when| CalEvent {
        uid: EventUid("x".to_owned()),
        title: title.to_owned(),
        when,
        location: None,
        calendar: CalendarName("std".to_owned()),
    };
    let now = at(26, 10, 0);
    let lines = |title: &str, when: &str| EventLines {
        title: title.to_owned(),
        when: when.to_owned(),
    };
    let cases = [
        (
            event(
                "Standup",
                EventTime::Timed {
                    start: at(26, 10, 30),
                    end: at(26, 11, 0),
                },
            ),
            lines("Standup", "10:30 – 11:00"),
        ),
        (
            event(
                "",
                EventTime::Timed {
                    start: at(26, 9, 30),
                    end: at(26, 11, 0),
                },
            ),
            lines("Untitled", "Now"),
        ),
        (
            event(
                "Dentist",
                EventTime::Timed {
                    start: at(27, 9, 0),
                    end: at(27, 10, 0),
                },
            ),
            lines("Dentist", "Tomorrow 09:00"),
        ),
        (
            event(
                "Holiday",
                EventTime::AllDay {
                    first: date(2026, 9, 26),
                    last: date(2026, 9, 26),
                },
            ),
            lines("Holiday", "All day"),
        ),
        (
            event(
                "Trip",
                EventTime::Timed {
                    start: at(28, 9, 0),
                    end: at(28, 10, 0),
                },
            ),
            lines("Trip", "Mon 09:00"),
        ),
    ];
    for (event, want) in cases {
        assert_eq!(
            event_lines(&event, now, &utc, ClockHours::H24),
            want,
            "{}",
            event.title
        );
    }
}
