//! Calendar events for the notification center's Calendar and Up Next widgets (design/20 §1.12,
//! moved into the widgets as macOS 11 did).
//!
//! Where events come from:
//!
//! - **An `.ics` file or a directory of them** is the cheap, local, dependency-free source, and
//!   it covers more than it sounds: vdirsyncer syncs any CalDAV server (Nextcloud, Fastmail,
//!   iCloud, Google) into a directory of one `.ics` per event per collection, the layout khal
//!   reads (`~/.calendars/<collection>/<uid>.ics` by convention); and every calendar exports
//!   `.ics`. [`IcsProvider`] reads them: `ics` parses RFC 5545 by hand (unfolding, `VEVENT`,
//!   `DTSTART`/`DTEND`/`DURATION`, `VALUE=DATE`, UTC and `TZID=` times resolved with jiff's zone
//!   database), and `rule` expands the common `RRULE` subset (`FREQ` daily to yearly,
//!   `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` on weekly rules, `EXDATE`).
//! - **evolution-data-server** (`org.gnome.evolution.dataserver.Sources5` and `Calendar8` on the
//!   session bus) is what GNOME Calendar and GNOME Online Accounts feed; it answers
//!   `GetObjectList` with iCalendar text, so the same parser would read it. It is not installed
//!   here (nor is GOA, whose calendars reach clients only through EDS), so it is a later
//!   provider beside it (an enum of providers).
//!
//! Dependencies: none added. `icalendar` brings chrono and expands no rule; `rrule` brings chrono
//! and chrono-tz (a second compiled-in zone database beside jiff's). jiff is already here.
//!
//! The service is passive until the daemon's first [`EventsCommand::Sources`] (from
//! `calendar.sources`), so a test or a headless render never reads a calendar. It publishes an
//! `Agenda`: today's date (republished at each local midnight, so the month grid moves on),
//! the events in `held_window` around today, and how the sources read.

mod backend;

use std::sync::Arc;

use sill_model::events::{Agenda, AutoEnv, EventsCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::{IcsProvider, ProviderRead, provider_for};

/// Calendar events as a [`Service`], found under `env`'s directories and dated by its clock.
/// Until the first `Sources` the agenda holds today only, with `SourceStatus::Off`.
#[derive(Debug)]
pub enum EventsService {}

impl Service for EventsService {
    const NAME: &'static str = "events";
    type State = Agenda;
    type Command = EventsCommand;
    type Deps = ();

    fn initial(env: &Env, (): &()) -> Agenda {
        Agenda::starting(env.clock.now().date())
    }

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach {
            auto: AutoEnv {
                home: env.dirs.home.clone(),
                data_home: env.dirs.data.clone(),
            },
            clock: Arc::clone(&env.clock),
        };
        async move {
            backend::run(io.state, io.commands, reach).await;
            Ok(())
        }
    }
}
