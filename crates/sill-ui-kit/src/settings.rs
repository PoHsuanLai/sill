//! The settings as a surface receives them: one more `Root::context` value beside
//! `crate::services::Services`, read by [`crate::hooks::use_settings`],
//! [`crate::hooks::use_section`], [`crate::hooks::use_styles`] and
//! [`crate::hooks::use_appearance`].

use ds::style::kit::Styles;
use ds_blitz::StylesGuard;
use ds_settings::Environment;
use sill_settings::ShellFile;
use tokio::sync::watch;

use crate::links::Outbox;

/// The person pressed Turn On Again on the notice that says their styles were turned off. The
/// daemon follows the [`SettingsFeed`]'s `turn_on` and sets `appearance.custom_styles` back to
/// `use`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnOnStyles;

/// Every settled `settings.toml` and the styles. Watch receivers, never Dioxus signals.
#[derive(Debug, Clone)]
pub struct SettingsFeed {
    /// The settled `settings.toml`.
    pub settings: watch::Receiver<ShellFile>,
    /// The settled styles: the desktop-wide `quire/style.css` and sill's own `style.css`, and the
    /// look (empty until the look setting lands), with the folder each may read files from.
    pub styles: watch::Receiver<Styles>,
    /// The crash guard of this start's styles; none in a render that has no guard.
    pub guard: watch::Receiver<Option<StylesGuard>>,
    /// Where the notice says Turn On Again; the daemon follows it.
    pub turn_on: Outbox<TurnOnStyles>,
    /// The settled appearance: `quire/appearance.toml` and the desktop's preferences. The daemon
    /// owns the one watch on both; [`crate::hooks::use_appearance`] reads this.
    pub appearance: watch::Receiver<Environment>,
}

impl SettingsFeed {
    /// A feed that never changes, for a headless render or a test: `file`, no styles, no guard.
    pub fn fixed(file: ShellFile) -> SettingsFeed {
        let (_tx, settings) = watch::channel(file);
        let (_tx, styles) = watch::channel(Styles::default());
        let (_tx, guard) = watch::channel(None);
        let (_tx, appearance) = watch::channel(Environment::default());
        SettingsFeed {
            settings,
            styles,
            guard,
            turn_on: Outbox::default(),
            appearance,
        }
    }
}
