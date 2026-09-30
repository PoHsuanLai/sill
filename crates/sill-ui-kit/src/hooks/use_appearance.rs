//! `use_appearance`: quire's appearance, from `ds-settings`, unless the root pins one.

use std::sync::Arc;

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook, use_signal};
use ds::base::spawner::Spawner;
use ds_blitz::TokioSpawner;
use ds_settings::{AppName, ConfigRoot, Environment, Store, SystemPrefsSource, use_environment};

/// An appearance a root pins in place of the desktop's: a test or a headless render provides it
/// as a root context so its surfaces never read the settings portal or `quire/appearance.toml`.
/// `use_environment` does both off the harness's thread, on wall time, so under a virtual clock
/// its replies land whenever the machine gets to them, inside a settle's quiet window too
/// and what they return is the developer's own desktop, not the test's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PinnedAppearance(pub Environment);

/// `quire/appearance.toml` and the settings portal, live; or the root's [`PinnedAppearance`].
pub fn use_appearance() -> ReadSignal<Environment> {
    // Read once, so the branch below (and the hooks it calls) is the same on every render.
    let pinned = use_hook(try_consume_context::<PinnedAppearance>);
    match pinned {
        Some(PinnedAppearance(environment)) => ReadSignal::new(use_signal(move || environment)),
        None => {
            // The runtime the host entered on this thread runs the file and portal watches.
            let spawner: Arc<dyn Spawner> = use_hook(|| Arc::new(TokioSpawner::current()));
            let store = Store::new(ConfigRoot::Xdg, AppName::QUIRE);
            use_environment(store, SystemPrefsSource::Portal, spawner)
        }
    }
}
