//! What the events service publishes.

use std::fmt::Debug;
use std::path::PathBuf;

use jiff::civil::Date;
use jiff::{Timestamp, ToSpan};

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

/// The days the service keeps events for around `today`: five weeks back (the month grid's
/// previous month) and ten weeks ahead (the next month and Up Next's longest reach).
pub fn held_window(today: Date) -> Window {
    Window {
        from: today.saturating_sub(35.days()),
        until: today.saturating_add(70.days()),
    }
}

/// Why one calendar could not be read. Its events are left out; the others are kept.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventsError {
    /// The file or directory could not be read (missing, unreadable, not UTF-8).
    #[error("reading {path}: {reason}")]
    Unreadable { path: PathBuf, reason: String },
    /// The text is not an iCalendar stream (no `BEGIN:VCALENDAR`, an unclosed component, a
    /// property line without `:`) at `line` (1-based, after unfolding).
    #[error("{path}, line {line}: {reason}")]
    Malformed {
        path: PathBuf,
        line: u32,
        reason: String,
    },
}
