//! What the events service hears and what it can be told.

use std::fmt::Debug;

use jiff::civil::Date;

use super::{CalEvent, EventSources, EventsError};

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

/// What the events service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventsCommand {
    /// Read from these sources from now on (the daemon sends `calendar.sources` at start and on
    /// every change; the first one ends the passive start).
    Sources(EventSources),
    /// Read every source again now (after a resume).
    Refresh,
}
