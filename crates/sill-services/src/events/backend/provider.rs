//! The seam between the events service and where events live: [`EventsProvider`], with
//! [`IcsProvider`] (files and vdir directories) and [`NullProvider`] (no calendars) now, and an
//! evolution-data-server provider later (module doc of `events`).

use std::fmt::Debug;
use std::path::PathBuf;

use jiff::tz::TimeZone;
use sill_model::events::{
    AutoEnv, CalEvent, EventSources, EventsError, IcsPlace, Window, auto_places,
};

/// Somewhere events can be read from. Blocking: the service calls it on the blocking pool.
pub trait EventsProvider: Send + Sync + Debug {
    /// Every occurrence overlapping `window`, all-day events as dates and timed ones resolved
    /// against their own `TZID` (a floating time in `zone`); how many calendars were read; and
    /// the calendars that failed.
    fn read(&self, window: Window, zone: &TimeZone) -> ProviderRead;

    /// The files and directories whose change should trigger a re-read (a `notify` watch on
    /// each; a directory's own entries and its collections' too).
    fn watched(&self) -> Vec<PathBuf>;
}

/// What one [`EventsProvider::read`] found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderRead {
    pub events: Vec<CalEvent>,
    pub calendars: u16,
    pub problems: Vec<EventsError>,
}

/// No calendars: `calendar.sources = "off"`, or `auto` found none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NullProvider;

impl EventsProvider for NullProvider {
    fn read(&self, _window: Window, _zone: &TimeZone) -> ProviderRead {
        ProviderRead::default()
    }

    fn watched(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

/// `.ics` files and directories of them. A file is one calendar, named by its stem. A directory
/// is one calendar when it holds `*.ics` files directly (a vdir collection: one event per file),
/// and each subdirectory that does is one more (vdirsyncer's `~/.calendars/<collection>/`),
/// named by the directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcsProvider {
    pub places: Vec<IcsPlace>,
}

impl EventsProvider for IcsProvider {
    fn read(&self, window: Window, zone: &TimeZone) -> ProviderRead {
        super::read::read_places(&self.places, window, zone)
    }

    fn watched(&self) -> Vec<PathBuf> {
        self.places
            .iter()
            .map(|place| match place {
                IcsPlace::File(path) | IcsPlace::Dir(path) | IcsPlace::Vdir(path) => path.clone(),
            })
            .collect()
    }
}

/// The provider `sources` names: `Off` is the null provider, `Places` an ICS provider over them
/// (a missing one is a problem the reader reports), and `Auto` an ICS provider over the
/// [`auto_places`] that `exists` (the null provider when there are none).
pub fn provider_for(
    sources: &EventSources,
    env: &AutoEnv,
    exists: impl Fn(&IcsPlace) -> bool,
) -> Box<dyn EventsProvider> {
    let places = match sources {
        EventSources::Off => Vec::new(),
        EventSources::Auto => auto_places(env).into_iter().filter(|p| exists(p)).collect(),
        EventSources::Places(places) => places.clone(),
    };
    match places.is_empty() {
        true => Box::new(NullProvider),
        false => Box::new(IcsProvider { places }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::{EventsProvider, IcsProvider, NullProvider, ProviderRead};
    use sill_model::events::IcsPlace;
    use sill_model::events::Window;

    fn window() -> Window {
        Window {
            from: date(2026, 9, 1),
            until: date(2026, 11, 1),
        }
    }

    #[test]
    fn the_null_provider_reads_nothing_and_watches_nothing() {
        assert_eq!(
            NullProvider.read(window(), &TimeZone::UTC),
            ProviderRead::default()
        );
        assert!(NullProvider.watched().is_empty());
    }

    #[test]
    fn an_ics_provider_watches_its_places() {
        let provider = IcsProvider {
            places: vec![
                IcsPlace::File(PathBuf::from("/c/std.ics")),
                IcsPlace::Dir(PathBuf::from("/c/dav")),
            ],
        };
        assert_eq!(
            provider.watched(),
            vec![PathBuf::from("/c/std.ics"), PathBuf::from("/c/dav")]
        );
    }

    /// A file and a vdir collection in a temp dir are two calendars; a broken file in the
    /// collection is one problem and costs only its own event.
    #[test]
    fn an_ics_provider_reads_files_and_collections() {
        let dir = std::env::temp_dir().join(format!("sill-ics-{}", std::process::id()));
        let collection = dir.join("dav");
        std::fs::create_dir_all(&collection).unwrap_or_else(|e| panic!("{e}"));
        let event = |uid: &str, day: &str| {
            format!(
                "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nSUMMARY:{uid}\r\nDTSTART;VALUE=DATE:{day}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
            )
        };
        let write = |path: PathBuf, text: String| {
            std::fs::write(path, text).unwrap_or_else(|e| panic!("{e}"));
        };
        write(dir.join("std.ics"), event("one", "20260927"));
        write(collection.join("two.ics"), event("two", "20261001"));
        write(collection.join("bad.ics"), "BEGIN:VCALENDAR\r\n".to_owned());
        let provider = IcsProvider {
            places: vec![
                IcsPlace::File(dir.join("std.ics")),
                IcsPlace::Dir(collection),
            ],
        };
        let read = provider.read(window(), &TimeZone::UTC);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read.calendars, 2);
        assert_eq!(read.events.len(), 2);
        assert_eq!(read.problems.len(), 1);
    }
}
