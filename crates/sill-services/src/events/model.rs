//! What the events service publishes, and its one reducer.

use jiff::civil::Date;
use jiff::{Timestamp, ToSpan};

use super::provider::EventsError;

/// An event's `UID` (an occurrence of a repeating event shares its series' uid).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct EventUid(pub String);

/// The calendar an event came from: the file's or the collection directory's stem.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct CalendarName(pub String);

/// When an event happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventTime {
    /// Whole days, `first` to `last` inclusive, in whatever zone the viewer is in.
    AllDay { first: Date, last: Date },
    /// An instant span; `end` is never before `start`.
    Timed { start: Timestamp, end: Timestamp },
}

/// One occurrence of one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalEvent {
    pub uid: EventUid,
    /// `SUMMARY`, unescaped; "Untitled" is the widget's to say, not this.
    pub title: String,
    pub when: EventTime,
    /// `LOCATION`, unescaped, when there is one.
    pub location: Option<String>,
    pub calendar: CalendarName,
}

/// A span of days, `from` inclusive, `until` exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Window {
    pub from: Date,
    pub until: Date,
}

/// How the sources read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceStatus {
    /// Not told any sources yet, or `calendar.sources = "off"`: the widgets show the month and
    /// no events.
    #[default]
    Off,
    /// `auto` found no calendar.
    NoCalendars,
    /// Read `calendars` calendars; `problems` names each file that could not be read or parsed
    /// (its events are left out, the rest are kept).
    Read {
        calendars: u16,
        problems: Vec<EventsError>,
    },
}

/// Today, the events around it, and how they were read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Agenda {
    /// The local date, republished at each local midnight.
    pub today: Date,
    /// Every occurrence overlapping [`held_window`]`(today)`, sorted by start.
    pub events: Vec<CalEvent>,
    pub status: SourceStatus,
}

impl Agenda {
    /// Today and nothing else, before any source is read.
    pub fn starting(today: Date) -> Agenda {
        Agenda {
            today,
            events: Vec::new(),
            status: SourceStatus::Off,
        }
    }
}

/// What changes an agenda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgendaEvent {
    /// The local date changed (midnight, a resume, a time-zone change).
    Day(Date),
    /// The sources were read: every occurrence found in the window, the calendars read, and the
    /// files that failed.
    Read {
        events: Vec<CalEvent>,
        calendars: u16,
        problems: Vec<EventsError>,
    },
    /// `auto` resolved to nothing.
    NoCalendars,
    /// The sources were switched off.
    Off,
}

/// The one place an [`Agenda`] changes. `Read` sorts the events by start (all-day before timed
/// on the same day, then by title) and drops duplicates of the same uid and time; `Off` and
/// `NoCalendars` drop every event; `Day` keeps the events (the next read moves the window).
///
/// The order is zone-free (the agenda is published to every surface): an all-day event sorts as
/// its first day's UTC midnight; the widgets order by their own day ([`super::up_next`]).
pub fn reduce_agenda(agenda: Agenda, event: AgendaEvent) -> Agenda {
    match event {
        AgendaEvent::Day(today) => Agenda { today, ..agenda },
        AgendaEvent::Read {
            events,
            calendars,
            problems,
        } => Agenda {
            events: sorted_once(events),
            status: SourceStatus::Read {
                calendars,
                problems,
            },
            ..agenda
        },
        AgendaEvent::NoCalendars => Agenda {
            events: Vec::new(),
            status: SourceStatus::NoCalendars,
            ..agenda
        },
        AgendaEvent::Off => Agenda {
            events: Vec::new(),
            status: SourceStatus::Off,
            ..agenda
        },
    }
}

/// `events` in start order, each uid and time once.
fn sorted_once(mut events: Vec<CalEvent>) -> Vec<CalEvent> {
    events.sort_by(|a, b| {
        order_key(a)
            .cmp(&order_key(b))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.uid.cmp(&b.uid))
    });
    let mut seen = std::collections::HashSet::new();
    events.retain(|event| seen.insert((event.uid.clone(), event.when)));
    events
}

/// Where an event sorts: its start, an all-day event at its first day's UTC midnight and before
/// a timed one at the same instant.
fn order_key(event: &CalEvent) -> (Timestamp, u8) {
    match event.when {
        EventTime::AllDay { first, .. } => (
            first
                .to_zoned(jiff::tz::TimeZone::UTC)
                .map_or(Timestamp::MIN, |zoned| zoned.timestamp()),
            0,
        ),
        EventTime::Timed { start, .. } => (start, 1),
    }
}

/// The days the service keeps events for around `today`: five weeks back (the month grid's
/// previous month) and ten weeks ahead (the next month and Up Next's longest reach).
pub fn held_window(today: Date) -> Window {
    Window {
        from: today.saturating_sub(35.days()),
        until: today.saturating_add(70.days()),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{
        Agenda, AgendaEvent, CalEvent, CalendarName, EventTime, EventUid, SourceStatus, Window,
        held_window, reduce_agenda,
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
        let read = reduce_agenda(
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
        let moved = reduce_agenda(read.clone(), AgendaEvent::Day(date(2026, 9, 27)));
        assert_eq!(moved.today, date(2026, 9, 27));
        assert_eq!(moved.events, read.events);
        let off = reduce_agenda(read, AgendaEvent::Off);
        assert_eq!(off.events, Vec::new());
        assert_eq!(off.status, SourceStatus::Off);
        let none = reduce_agenda(start, AgendaEvent::NoCalendars);
        assert_eq!(none.status, SourceStatus::NoCalendars);
    }
}
