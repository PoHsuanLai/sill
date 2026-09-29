//! The events state's pure transition.

use jiff::Timestamp;

use super::{Agenda, AgendaEvent, CalEvent, EventTime, SourceStatus};

/// The one place an [`Agenda`] changes. `Read` sorts the events by start (all-day before timed
/// on the same day, then by title) and drops duplicates of the same uid and time; `Off` and
/// `NoCalendars` drop every event; `Day` keeps the events (the next read moves the window).
///
/// The order is zone-free (the agenda is published to every surface): an all-day event sorts as
/// its first day's UTC midnight; the widgets order by their own day ([`super::up_next`]).
pub fn step(agenda: Agenda, event: AgendaEvent) -> Agenda {
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
