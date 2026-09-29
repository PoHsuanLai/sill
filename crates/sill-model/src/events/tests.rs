use jiff::civil::date;

use super::{
    Agenda, AgendaEvent, CalEvent, CalendarName, EventTime, EventUid, SourceStatus, Window,
    held_window, step,
};

fn all_day(uid: &str, title: &str, day: i8) -> CalEvent {
    CalEvent {
        uid: EventUid(uid.to_owned()),
        title: title.to_owned(),
        when: EventTime::AllDay {
            first: date(2026, 9, day),
            last: date(2026, 9, day),
        },
        location: None,
        calendar: CalendarName("std".to_owned()),
    }
}

#[test]
fn the_window_reaches_five_weeks_back_and_ten_ahead() {
    assert_eq!(
        held_window(date(2026, 9, 26)),
        Window {
            from: date(2026, 8, 22),
            until: date(2026, 12, 5),
        }
    );
}

#[test]
fn a_read_sorts_and_a_switch_off_empties() {
    let start = Agenda::starting(date(2026, 9, 26));
    let read = step(
        start.clone(),
        AgendaEvent::Read {
            events: vec![
                all_day("b", "B", 28),
                all_day("a", "A", 27),
                all_day("a", "A", 27),
            ],
            calendars: 1,
            problems: Vec::new(),
        },
    );
    assert_eq!(
        read.events,
        vec![all_day("a", "A", 27), all_day("b", "B", 28)]
    );
    assert_eq!(
        read.status,
        SourceStatus::Read {
            calendars: 1,
            problems: Vec::new()
        }
    );
    let moved = step(read.clone(), AgendaEvent::Day(date(2026, 9, 27)));
    assert_eq!(moved.today, date(2026, 9, 27));
    assert_eq!(moved.events, read.events);
    let off = step(read, AgendaEvent::Off);
    assert_eq!(off.events, Vec::new());
    assert_eq!(off.status, SourceStatus::Off);
    let none = step(start, AgendaEvent::NoCalendars);
    assert_eq!(none.status, SourceStatus::NoCalendars);
}
