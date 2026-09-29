//! The events service's sources from `calendar.sources`: sent at start and on every change
//! ([`super::follow`]), the first one ending the service's passive start.

use std::path::{Path, PathBuf};

use sill_model::events::{EventSources, EventsCommand, parse_sources};
use sill_settings::ShellFile;

use super::Policy;

/// The sources `file` names (`parse_sources` against `home`).
pub fn sources_of(file: &ShellFile, home: &Path) -> EventSources {
    parse_sources(&file.calendar.sources, home)
}

/// The events service's sources from the settings, found against `home`.
#[derive(Debug, Clone)]
pub struct CalendarBridge {
    pub home: PathBuf,
}

impl Policy for CalendarBridge {
    type Command = EventsCommand;
    type Out = EventSources;

    fn of(&self, file: &ShellFile) -> EventSources {
        sources_of(file, &self.home)
    }

    fn command(sources: EventSources) -> EventsCommand {
        EventsCommand::Sources(sources)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::events::EventSources;
    use sill_settings::ShellFile;

    use std::path::Path;

    use super::sources_of;

    #[test]
    fn the_default_is_auto_and_off_is_off() {
        let home = Path::new("/home/p");
        assert_eq!(sources_of(&ShellFile::default(), home), EventSources::Auto);
        let mut off = ShellFile::default();
        off.calendar.sources = "off".to_owned();
        assert_eq!(sources_of(&off, home), EventSources::Off);
    }
}
