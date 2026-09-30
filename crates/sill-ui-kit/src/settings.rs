//! The settings as a surface receives them: one more `Root::context` value beside
//! `crate::services::Services`, read by [`crate::hooks::use_settings`],
//! [`crate::hooks::use_section`] and [`crate::hooks::use_user_style`].

use ds_settings::UserStyle;
use sill_settings::ShellFile;
use tokio::sync::watch;

/// Every settled `settings.toml` and `style.css`. Watch receivers, never Dioxus signals.
#[derive(Debug, Clone)]
pub struct SettingsFeed {
    /// The settled `settings.toml`.
    pub settings: watch::Receiver<ShellFile>,
    /// The settled `style.css`: the person's own stylesheet, empty when there is none.
    pub user_style: watch::Receiver<UserStyle>,
}

impl SettingsFeed {
    /// A feed that never changes, for a headless render or a test: `file` and no user style.
    pub fn fixed(file: ShellFile) -> SettingsFeed {
        let (_tx, settings) = watch::channel(file);
        let (_tx, user_style) = watch::channel(UserStyle::default());
        SettingsFeed {
            settings,
            user_style,
        }
    }
}
