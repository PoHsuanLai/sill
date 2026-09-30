//! The settings the daemon holds: the one watch channel every surface and policy follows, fed by
//! the file watch and by `sill reload`.

use ds_settings::Store;
use shell_host::HostEvent;
use sill_settings::ShellFile;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

/// Re-read `settings.toml` from `store` and publish it (the bar and every surface pick it up
/// through the settings feed and, for geometry, through `ShellMsg::Settings`).
pub fn reload(store: &Store, settings: &watch::Sender<ShellFile>) {
    settings.send_replace(super::store::load(store));
}

/// `event`'s file, published to `settings_tx`, if it is a watched settings change
/// (`ShellMsg::Settings`, from the file watch task in `daemon::run`); anything else is a no-op. The same
/// act [`reload`] performs for `sill reload`, so a plain `settings.toml` edit (or one of
/// sill's own writes: `crate::edit_widgets`, `super::focus`) takes effect without one.
/// `ds_settings::Store::watch` already debounces a burst of writes to one settled file and keeps the
/// last good file on a parse failure, so there is nothing further to coalesce or guard here.
pub fn publish_watched(settings_tx: &watch::Sender<ShellFile>, event: &HostEvent<ShellMsg>) {
    if let HostEvent::App(ShellMsg::Settings(file)) = event {
        settings_tx.send_replace((**file).clone());
    }
}

/// [`publish_watched`] (a pure change-to-dispatch mapping: only `ShellMsg::Settings`
/// dispatches, everything else is a no-op) and the live watch it feeds, end to end through a
/// temp config dir.
#[cfg(test)]
mod settings_publish_tests {
    use std::time::Duration;

    use ds_blitz::TokioSpawner;
    use shell_host::HostEvent;
    use sill_settings::ShellFile;
    use tokio::sync::watch;

    use super::publish_watched;
    use crate::daemon::msg::ShellMsg;
    use crate::daemon::settings::store::store;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sill-daemon-settings-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        dir
    }

    #[test]
    fn a_watched_settings_event_publishes_its_file() {
        let (tx, rx) = watch::channel(ShellFile::default());
        let mut changed = ShellFile::default();
        changed.notifications.dnd = sill_settings::Dnd::On;
        let event = HostEvent::App(ShellMsg::Settings(Box::new(changed.clone())));
        publish_watched(&tx, &event);
        assert_eq!(rx.borrow().clone(), changed);
    }

    #[test]
    fn every_other_event_leaves_the_feed_alone() {
        let (tx, rx) = watch::channel(ShellFile::default());
        for event in [
            HostEvent::Started,
            HostEvent::App(ShellMsg::Dim(sill_model::idle::DimShown::Clear)),
        ] {
            publish_watched(&tx, &event);
        }
        assert_eq!(rx.borrow().clone(), ShellFile::default());
    }

    /// The regression itself: a `settings.toml` edit under a temp config dir, with no `sill
    /// reload`, reaches a `SettingsFeed`-shaped channel through the same watch and the same
    /// `publish_watched` the host event handler calls.
    #[tokio::test]
    async fn a_file_write_reaches_the_feed_without_a_reload() {
        let dir = temp_dir("live");
        let store = store(&dir);
        store
            .save(&ShellFile::default())
            .unwrap_or_else(|e| panic!("{e}"));
        let mut shell_watch = store.watch::<ShellFile>(&TokioSpawner::current());

        let (settings_tx, settings_rx) = watch::channel(ShellFile::default());

        let mut changed = ShellFile::default();
        changed.notifications.dnd = sill_settings::Dnd::On;
        store.save(&changed).unwrap_or_else(|e| panic!("{e}"));

        let loaded = tokio::time::timeout(Duration::from_secs(2), shell_watch.changed())
            .await
            .unwrap_or_else(|_| panic!("no settled change within 2s"))
            .unwrap_or_else(|| panic!("the watch stopped"));
        let event = HostEvent::App(ShellMsg::Settings(Box::new(loaded.value)));
        publish_watched(&settings_tx, &event);

        assert_eq!(settings_rx.borrow().clone(), changed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
