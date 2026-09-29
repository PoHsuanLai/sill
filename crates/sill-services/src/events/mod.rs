//! Calendar events for the notification center's Calendar and Up Next widgets (design/20 §1.12,
//! moved into the widgets as macOS 11 did).
//!
//! Where events come from:
//!
//! - **An `.ics` file or a directory of them** is the cheap, local, dependency-free source, and
//!   it covers more than it sounds: vdirsyncer syncs any CalDAV server (Nextcloud, Fastmail,
//!   iCloud, Google) into a directory of one `.ics` per event per collection, the layout khal
//!   reads (`~/.calendars/<collection>/<uid>.ics` by convention); and every calendar exports
//!   `.ics`. [`IcsProvider`] reads them: [`ics`] parses RFC 5545 by hand (unfolding, `VEVENT`,
//!   `DTSTART`/`DTEND`/`DURATION`, `VALUE=DATE`, UTC and `TZID=` times resolved with jiff's zone
//!   database), and [`rule`] expands the common `RRULE` subset (`FREQ` daily to yearly,
//!   `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` on weekly rules, `EXDATE`).
//! - **evolution-data-server** (`org.gnome.evolution.dataserver.Sources5` and `Calendar8` on the
//!   session bus) is what GNOME Calendar and GNOME Online Accounts feed; it answers
//!   `GetObjectList` with iCalendar text, so the same parser would read it. It is not installed
//!   here (nor is GOA, whose calendars reach clients only through EDS), so it is a later
//!   provider behind the same trait.
//!
//! Dependencies: none added. `icalendar` brings chrono and expands no rule; `rrule` brings chrono
//! and chrono-tz (a second compiled-in zone database beside jiff's). jiff is already here.
//!
//! The service is passive until the daemon's first [`EventsCommand::Sources`] (from
//! `calendar.sources`), so a test or a headless render never reads a calendar. It publishes an
//! [`Agenda`]: today's date (republished at each local midnight, so the month grid moves on),
//! the events in [`held_window`] around today, and how the sources read.

pub mod agenda;
pub mod ics;
mod ics_line;
mod ics_value;
mod live;
mod live_watch;
pub mod model;
mod occur;
pub mod provider;
mod read;
pub mod rule;
mod rule_expand;
pub mod sources;

pub use agenda::{Days, up_next};
pub use model::{
    Agenda, AgendaEvent, CalEvent, CalendarName, EventTime, EventUid, SourceStatus, Window,
    held_window, reduce_agenda,
};
pub use provider::{
    EventsError, EventsProvider, IcsProvider, NullProvider, ProviderRead, provider_for,
};
pub use sources::{AutoEnv, EventSources, IcsPlace, auto_places, parse_sources};

use jiff::Zoned;
use jiff::tz::TimeZone;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the events service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventsCommand {
    /// Read from these sources from now on (the daemon sends `calendar.sources` at start and on
    /// every change; the first one ends the passive start).
    Sources(EventSources),
    /// Read every source again now (after a resume).
    Refresh,
}

/// Start the events task on `rt`. Until the first `Sources` the agenda holds today only, with
/// `SourceStatus::Off`.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Agenda>, Control<EventsCommand>) {
    let (tx, rx) = watch::channel(Agenda::starting(local_today()));
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}

/// The local calendar date now, with the system time zone read afresh.
pub(crate) fn local_today() -> jiff::civil::Date {
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    Zoned::now().with_time_zone(zone).date()
}
