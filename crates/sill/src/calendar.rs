//! The events service's sources from `calendar.sources` (FINDINGS "M10 freeze"; M10 lane a fills
//! the stubs): sent at start and on every change, the first one ending the service's passive
//! start. Until lane a fills it, the service is never told and reads no calendar.

use sill_services::events::EventSources;
use sill_services::{Control, EventsCommand};
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::watch;

/// The sources `file` names (`parse_sources` against `$HOME`).
pub fn sources_of(file: &ShellFile) -> EventSources {
    let _ = file;
    todo!("M10 fill: a-calendar")
}

/// Send [`sources_of`] now and whenever it changes, on `rt`, until either side is gone.
pub fn spawn_policy(
    rt: &Handle,
    settings: watch::Receiver<ShellFile>,
    control: Control<EventsCommand>,
) {
    let _ = (rt, settings, control);
}

#[cfg(test)]
mod tests {
    use sill_services::events::EventSources;
    use sill_settings::ShellFile;

    use super::sources_of;

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn the_default_is_auto_and_off_is_off() {
        assert_eq!(sources_of(&ShellFile::default()), EventSources::Auto);
        let mut off = ShellFile::default();
        off.calendar.sources = "off".to_owned();
        assert_eq!(sources_of(&off), EventSources::Off);
    }
}
