//! The desktop's appearance: `quire/appearance.toml` and the settings portal, watched once for
//! the whole shell and published to the feed every surface root draws it from (ARCHITECTURE.md
//! section 8). Like the other watches, the task sends each settled value to the host thread,
//! which publishes it.

use ds_blitz::TokioSpawner;
use ds_settings::{
    AppName, AppearanceFile, ConfigRoot, Environment, Store, SystemPrefsSource, SystemPrefsWatch,
};
use shell_host::{HostEvent, HostHandle};
use tokio::runtime::Runtime;
use tokio::sync::watch;

use super::store::report;
use crate::daemon::msg::ShellMsg;

/// The store of `quire/appearance.toml`, which every quire app shares.
pub fn appearance_store() -> Store {
    Store::new(ConfigRoot::Xdg, AppName::QUIRE)
}

/// The appearance as the file has it now, the desktop's preferences at their defaults until the
/// portal has answered.
pub fn load_appearance(store: &Store) -> Environment {
    let loaded = store.load::<AppearanceFile>();
    report(&loaded);
    Environment {
        settings: loaded.value,
        ..Environment::default()
    }
}

/// Watch `store` and the portal on `runtime`, sending the whole appearance to `host` at each
/// change. The file comes first, so a slow or absent portal never delays the person's choice.
pub fn spawn_appearance_watch(
    runtime: &Runtime,
    store: &Store,
    first: Environment,
    host: HostHandle<ShellMsg>,
) {
    let spawner = TokioSpawner::on(runtime.handle().clone());
    let mut files = store.watch::<AppearanceFile>(&spawner);
    runtime.spawn(async move {
        let mut prefs = SystemPrefsWatch::start(&SystemPrefsSource::Portal, &spawner).await;
        let mut now = Environment {
            system: prefs.current(),
            ..first
        };
        if host
            .send(ShellMsg::Appearance(Box::new(now.clone())))
            .is_err()
        {
            return;
        }
        loop {
            tokio::select! {
                Some(loaded) = files.changed() => {
                    report(&loaded);
                    now.settings = loaded.value;
                }
                Some(system) = prefs.changed() => now.system = system,
                else => break,
            }
            if host
                .send(ShellMsg::Appearance(Box::new(now.clone())))
                .is_err()
            {
                break;
            }
        }
    });
}

/// `event`'s appearance, published to `appearance`, if it is a watched change
/// (`ShellMsg::Appearance`); anything else is a no-op.
pub fn publish_watched_appearance(
    appearance: &watch::Sender<Environment>,
    event: &HostEvent<ShellMsg>,
) {
    if let HostEvent::App(ShellMsg::Appearance(now)) = event {
        appearance.send_replace((**now).clone());
    }
}

#[cfg(test)]
mod tests {
    use ds_settings::Environment;
    use shell_host::HostEvent;
    use tokio::sync::watch;

    use super::publish_watched_appearance;
    use crate::daemon::msg::ShellMsg;

    #[test]
    fn only_a_watched_appearance_reaches_the_feed() {
        let (tx, rx) = watch::channel(Environment::default());
        let mut changed = Environment::default();
        changed.system.scheme = ds::prelude::Scheme::Dark;
        publish_watched_appearance(&tx, &HostEvent::Started);
        assert_eq!(*rx.borrow(), Environment::default());
        publish_watched_appearance(
            &tx,
            &HostEvent::App(ShellMsg::Appearance(Box::new(changed.clone()))),
        );
        assert_eq!(*rx.borrow(), changed);
    }
}
