//! The events task: waits for `Sources`, builds the provider (`provider_for`, `auto` reading
//! `$HOME` and `$XDG_DATA_HOME`), reads it on the blocking pool over
//! `held_window(today)` in the system zone, watches `watched()` with `notify` (debounced),
//! re-reads on a change, a `Refresh` and the day's turn, and publishes through `reduce_agenda`.
//! Ends when every receiver is gone.

use std::sync::Arc;
use std::time::Duration;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{ToSpan, Zoned};
use tokio::sync::{mpsc, watch};

use super::EventsCommand;
use super::live_watch::Touches;
use super::model::{Agenda, AgendaEvent, held_window, reduce_agenda};
use super::provider::{EventsProvider, provider_for};
use super::sources::{AutoEnv, EventSources, IcsPlace, auto_places};

/// The longest the task sleeps without looking at the date: a resume or a zone change shows
/// within a minute.
const DAY_CHECK: Duration = Duration::from_secs(60);

pub(super) async fn run(
    tx: watch::Sender<Agenda>,
    mut commands: mpsc::UnboundedReceiver<EventsCommand>,
) {
    let mut sources = loop {
        match commands.recv().await {
            Some(EventsCommand::Sources(sources)) => break sources,
            Some(EventsCommand::Refresh) => continue,
            None => return,
        }
    };
    loop {
        match follow(&tx, &mut commands, &sources).await {
            Next::Switch(next) => sources = next,
            Next::Stop => return,
        }
    }
}

/// What ends following one set of sources.
enum Next {
    Switch(EventSources),
    Stop,
}

/// What the task reads for one set of sources.
enum Plan {
    Off,
    NoCalendars,
    Read(Arc<dyn EventsProvider>),
}

/// Publish `sources`' events and keep them fresh until the sources change or everyone leaves.
async fn follow(
    tx: &watch::Sender<Agenda>,
    commands: &mut mpsc::UnboundedReceiver<EventsCommand>,
    sources: &EventSources,
) -> Next {
    let plan = plan_for(sources).await;
    let watched = match &plan {
        Plan::Read(provider) => provider.watched(),
        Plan::Off | Plan::NoCalendars => Vec::new(),
    };
    let mut touches = Touches::watch(&watched);
    publish(tx, &plan).await;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                None => return Next::Stop,
                Some(EventsCommand::Sources(next)) if next != *sources => {
                    return Next::Switch(next);
                }
                Some(EventsCommand::Sources(_) | EventsCommand::Refresh) => publish(tx, &plan).await,
            },
            () = touches.settled() => publish(tx, &plan).await,
            () = tokio::time::sleep(until_day_check()) => {
                let today = super::local_today();
                if today != tx.borrow().today {
                    tx.send_modify(|agenda| {
                        *agenda = reduce_agenda(agenda.clone(), AgendaEvent::Day(today));
                    });
                    publish(tx, &plan).await;
                }
            }
            () = tx.closed() => return Next::Stop,
        }
    }
}

/// The provider `sources` names, `auto` resolved against this session's directories.
async fn plan_for(sources: &EventSources) -> Plan {
    let sources = sources.clone();
    let built = tokio::task::spawn_blocking(move || {
        let env = auto_env();
        let exists = |place: &IcsPlace| match place {
            IcsPlace::File(path) | IcsPlace::Dir(path) => path.exists(),
        };
        let found = auto_places(&env).iter().any(exists);
        match (&sources, found) {
            (EventSources::Off, _) => Plan::Off,
            (EventSources::Auto, false) => Plan::NoCalendars,
            _ => Plan::Read(Arc::from(provider_for(&sources, &env, exists))),
        }
    })
    .await;
    built.unwrap_or(Plan::Off)
}

/// What `auto` looks at in this session.
fn auto_env() -> AutoEnv {
    let home = dirs::home_dir().unwrap_or_default();
    let data_home = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));
    AutoEnv { home, data_home }
}

/// Read `plan` on the blocking pool and publish what it found.
async fn publish(tx: &watch::Sender<Agenda>, plan: &Plan) {
    let event = match plan {
        Plan::Off => AgendaEvent::Off,
        Plan::NoCalendars => AgendaEvent::NoCalendars,
        Plan::Read(provider) => {
            let provider = Arc::clone(provider);
            let today = tx.borrow().today;
            let read = tokio::task::spawn_blocking(move || {
                let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
                provider.read(held_window(today), &zone)
            })
            .await;
            match read {
                Ok(read) => AgendaEvent::Read {
                    events: read.events,
                    calendars: read.calendars,
                    problems: read.problems,
                },
                Err(_) => return,
            }
        }
    };
    tx.send_if_modified(|agenda| {
        let next = reduce_agenda(agenda.clone(), event);
        let changed = next != *agenda;
        *agenda = next;
        changed
    });
}

/// How long until the date should be looked at again: the next local midnight, at most a
/// minute away.
fn until_day_check() -> Duration {
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    let now = Zoned::now().with_time_zone(zone.clone());
    next_midnight(now.date(), &zone)
        .and_then(|midnight| Duration::try_from(now.timestamp().duration_until(midnight)).ok())
        .map_or(DAY_CHECK, |left| left.min(DAY_CHECK))
        .max(Duration::from_millis(10))
}

/// The instant the day after `today` starts in `zone`.
fn next_midnight(today: Date, zone: &TimeZone) -> Option<jiff::Timestamp> {
    today
        .checked_add(1.day())
        .ok()?
        .to_zoned(zone.clone())
        .ok()
        .map(|zoned| zoned.timestamp())
}
