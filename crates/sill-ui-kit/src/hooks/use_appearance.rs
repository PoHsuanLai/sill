//! `use_appearance`: the desktop's appearance, from the one feed the daemon owns, unless the
//! root pins one.

use dioxus::prelude::{ReadSignal, Signal, try_consume_context, use_hook, use_signal};
use ds_settings::Environment;
use shell_host::use_watch;

use crate::settings::SettingsFeed;

/// An appearance a root pins in place of the desktop's: a test or a headless render provides it
/// as a root context so its surfaces read neither the settings portal nor
/// `quire/appearance.toml`, and what they draw is the test's, not the developer's own desktop.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PinnedAppearance(pub Environment);

/// A pinned appearance a test can change while the surface runs, as the person does from the
/// control center: the harness provides it and holds the signal (`SurfaceHarness::set_appearance`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchableAppearance(pub Signal<Environment>);

/// The appearance every surface draws from: the root's [`SwitchableAppearance`] or
/// [`PinnedAppearance`], else the [`SettingsFeed`]'s `appearance` (the daemon's one watch of
/// `quire/appearance.toml` and the settings portal). A root with none of them gets
/// `Environment::default()`; no surface starts a watch of its own.
pub fn use_appearance() -> ReadSignal<Environment> {
    // Read once, so the branch below (and the hooks it calls) is the same on every render.
    let switchable = use_hook(try_consume_context::<SwitchableAppearance>);
    let pinned = use_hook(try_consume_context::<PinnedAppearance>);
    let feed = use_hook(try_consume_context::<SettingsFeed>);
    match (switchable, pinned, feed) {
        (Some(SwitchableAppearance(signal)), _, _) => ReadSignal::new(signal),
        (None, Some(PinnedAppearance(environment)), _) => {
            ReadSignal::new(use_signal(move || environment))
        }
        (None, None, Some(feed)) => use_watch(feed.appearance),
        (None, None, None) => ReadSignal::new(use_signal(Environment::default)),
    }
}
