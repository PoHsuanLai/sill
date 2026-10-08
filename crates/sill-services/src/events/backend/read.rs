//! What [`crate::events::IcsProvider`] reads: its places as calendars (a file; a vdir collection; a
//! directory of collections), each calendar's files parsed, overrides merged into their series,
//! and every occurrence in the window. Blocking: the service runs it on the blocking pool.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use jiff::tz::TimeZone;
use sill_model::events::{
    CalendarName, EventsError, IcsError, IcsEvent, IcsPlace, Window, is_task_list, occurrences,
    parse_ics, unsupported_parts,
};

use super::provider::ProviderRead;

/// One calendar: its name and its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Calendar {
    pub name: CalendarName,
    pub files: Vec<PathBuf>,
}

/// Read every place in `places` over `window`.
pub(crate) fn read_places(places: &[IcsPlace], window: Window, zone: &TimeZone) -> ProviderRead {
    let mut out = ProviderRead::default();
    for place in places {
        match calendars_in(place) {
            Ok(calendars) => {
                for calendar in calendars {
                    read_calendar(&calendar, window, zone, &mut out);
                }
            }
            Err(problem) => out.problems.push(problem),
        }
    }
    out
}

/// The calendars one place holds.
pub(crate) fn calendars_in(place: &IcsPlace) -> Result<Vec<Calendar>, EventsError> {
    match place {
        IcsPlace::File(path) => Ok(vec![Calendar {
            name: name_of(path),
            files: vec![path.clone()],
        }]),
        IcsPlace::Vdir(path) => {
            // An account directory that cannot be read costs that account only.
            let (_, accounts) = entries(path)?;
            Ok(accounts
                .iter()
                .filter_map(|account| entries(account).ok())
                .flat_map(|(_, collections)| collections)
                // `<collection>-tasks` holds VTODOs: a task list is not a calendar.
                .filter(|dir| !is_task_list(dir))
                .filter_map(|dir| {
                    let (files, _) = entries(&dir).ok()?;
                    Some(Calendar {
                        name: name_of(&dir),
                        files,
                    })
                })
                .filter(|calendar| !calendar.files.is_empty())
                .collect())
        }
        IcsPlace::Dir(path) => {
            let (files, dirs) = entries(path)?;
            let own = Some(Calendar {
                name: name_of(path),
                files,
            });
            let nested = dirs.into_iter().filter_map(|dir| {
                let (files, _) = entries(&dir).ok()?;
                Some(Calendar {
                    name: name_of(&dir),
                    files,
                })
            });
            Ok(own
                .into_iter()
                .chain(nested)
                .filter(|calendar| !calendar.files.is_empty())
                .collect())
        }
    }
}

/// A directory's `.ics` files and its subdirectories, sorted, hidden ones left out.
fn entries(dir: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), EventsError> {
    let listing = std::fs::read_dir(dir).map_err(|error| unreadable(dir, &error))?;
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in listing.flatten() {
        let path = entry.path();
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        let is_ics = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ics"));
        match (hidden, path.is_dir(), is_ics) {
            (true, _, _) => {}
            (false, true, _) => dirs.push(path),
            (false, false, true) => files.push(path),
            (false, false, false) => {}
        }
    }
    files.sort();
    dirs.sort();
    Ok((files, dirs))
}

/// A calendar's name: its file's stem or its directory's name.
fn name_of(path: &Path) -> CalendarName {
    let stem = path.file_stem().or_else(|| path.file_name());
    CalendarName(stem.map_or_else(String::new, |s| s.to_string_lossy().into_owned()))
}

/// Read one calendar into `out`: counted when any of its files parses.
fn read_calendar(calendar: &Calendar, window: Window, zone: &TimeZone, out: &mut ProviderRead) {
    let mut parsed: Vec<(PathBuf, IcsEvent)> = Vec::new();
    let mut any = Parsed::None;
    for path in &calendar.files {
        match read_file(path) {
            Ok(events) => {
                any = Parsed::Some;
                parsed.extend(events.into_iter().map(|event| (path.clone(), event)));
            }
            Err(problem) => out.problems.push(problem),
        }
    }
    if any == Parsed::Some {
        out.calendars = out.calendars.saturating_add(1);
    }
    for (path, event) in merge_overrides(parsed) {
        note_unsupported(&path, &event);
        out.events
            .extend(occurrences(&event, &calendar.name, window, zone));
    }
}

/// Whether any file of a calendar parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parsed {
    None,
    Some,
}

/// One file's events, or why it could not be read.
fn read_file(path: &Path) -> Result<Vec<IcsEvent>, EventsError> {
    let text = std::fs::read_to_string(path).map_err(|error| unreadable(path, &error))?;
    parse_ics(&text).map_err(|error| {
        let line = match &error {
            IcsError::NotCalendar => 1,
            IcsError::Unbalanced { line, .. } | IcsError::Bad { line, .. } => *line,
        };
        EventsError::Malformed {
            path: path.to_path_buf(),
            line,
            reason: error.to_string(),
        }
    })
}

fn unreadable(path: &Path, error: &std::io::Error) -> EventsError {
    EventsError::Unreadable {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

/// Every event with its overrides merged: an override's `RECURRENCE-ID` becomes an `EXDATE` of
/// its series (in the same calendar, any file), and the override stands as a single event.
pub(crate) fn merge_overrides(parsed: Vec<(PathBuf, IcsEvent)>) -> Vec<(PathBuf, IcsEvent)> {
    let (mut series, overrides): (Vec<_>, Vec<_>) = parsed
        .into_iter()
        .partition(|(_, event)| event.recurrence_id.is_none());
    let overrides = overrides.into_iter().map(|(path, mut event)| {
        if let (Some(master), Some(id)) = (
            series.iter_mut().find(|(_, s)| s.uid == event.uid),
            event.recurrence_id.clone(),
        ) {
            master.1.exdates.push(id);
        }
        event.rule = None;
        (path, event)
    });
    let overrides: Vec<_> = overrides.collect();
    series.extend(overrides);
    series
}

/// Log, once per process for each file, event and rule, a rule this subset shows once.
fn note_unsupported(path: &Path, event: &IcsEvent) {
    static LOGGED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    let Some(rule) = event.rule.as_ref() else {
        return;
    };
    let parts = unsupported_parts(rule);
    if parts.is_empty() {
        return;
    }
    let key = format!("{}\u{0}{}\u{0}{parts:?}", path.display(), event.uid);
    let mut logged = LOGGED.lock().unwrap_or_else(PoisonError::into_inner);
    if logged.get_or_insert_with(HashSet::new).insert(key) {
        eprintln!(
            "sill: events: {}: {:?} repeats by {}, which is not expanded; shown once",
            path.display(),
            event.summary,
            parts.join(";"),
        );
    }
}

/// Every occurrence the events hold, for a test that reads text rather than files.
#[cfg(test)]
pub(crate) fn occurrences_of(
    text: &str,
    calendar: &str,
    window: Window,
    zone: &TimeZone,
) -> Result<Vec<sill_model::events::CalEvent>, IcsError> {
    let events = parse_ics(text)?;
    let merged = merge_overrides(
        events
            .into_iter()
            .map(|event| (PathBuf::new(), event))
            .collect(),
    );
    let name = CalendarName(calendar.to_owned());
    Ok(merged
        .into_iter()
        .flat_map(|(_, event)| occurrences(&event, &name, window, zone))
        .collect())
}

#[cfg(test)]
mod tests;
