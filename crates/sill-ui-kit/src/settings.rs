//! The settings as a surface receives them: one more `Root::context` value beside
//! `crate::services::Services`, read by [`crate::hooks::use_settings`],
//! [`crate::hooks::use_section`], [`crate::hooks::use_user_style`] and
//! [`crate::hooks::use_appearance`].

use ds_settings::{Environment, UserStyle};
use sill_settings::ShellFile;
use tokio::sync::watch;

/// Every settled `settings.toml` and `style.css`. Watch receivers, never Dioxus signals.
#[derive(Debug, Clone)]
pub struct SettingsFeed {
    /// The settled `settings.toml`.
    pub settings: watch::Receiver<ShellFile>,
    /// The settled `style.css`: the person's own stylesheet, empty when there is none.
    pub user_style: watch::Receiver<UserStyle>,
    /// The settled appearance: `quire/appearance.toml` and the desktop's preferences. The daemon
    /// owns the one watch on both; [`crate::hooks::use_appearance`] reads this.
    pub appearance: watch::Receiver<Environment>,
}

impl SettingsFeed {
    /// A feed that never changes, for a headless render or a test: `file` and no user style.
    pub fn fixed(file: ShellFile) -> SettingsFeed {
        let (_tx, settings) = watch::channel(file);
        let (_tx, user_style) = watch::channel(UserStyle::default());
        let (_tx, appearance) = watch::channel(Environment::default());
        SettingsFeed {
            settings,
            user_style,
            appearance,
        }
    }
}
