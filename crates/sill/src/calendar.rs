//! The events service's sources from `calendar.sources`: sent at start and on every change, the
//! first one ending the service's passive start.

use std::path::PathBuf;

use sill_services::events::{EventSources, parse_sources};
use sill_services::{Control, EventsCommand};
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::watch;

/// The sources `file` names (`parse_sources` against `$HOME`).
pub fn sources_of(file: &ShellFile) -> EventSources {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    parse_sources(&file.calendar.sources, &home)
}

/// Send [`sources_of`] now and whenever it changes, on `rt`, until either side is gone.
pub fn spawn_policy(
    rt: &Handle,
    mut settings: watch::Receiver<ShellFile>,
    control: Control<EventsCommand>,
) {
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let sources = sources_of(&settings.borrow_and_update());
            if sent.as_ref() != Some(&sources) {
                if control
                    .send(EventsCommand::Sources(sources.clone()))
                    .is_err()
                {
                    return;
                }
                sent = Some(sources);
            }
            if settings.changed().await.is_err() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_services::events::EventSources;
    use sill_settings::ShellFile;

    use super::sources_of;

    #[test]
    fn the_default_is_auto_and_off_is_off() {
        assert_eq!(sources_of(&ShellFile::default()), EventSources::Auto);
        let mut off = ShellFile::default();
        off.calendar.sources = "off".to_owned();
        assert_eq!(sources_of(&off), EventSources::Off);
    }
}
