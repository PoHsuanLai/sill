//! The settings as a surface receives them: one more `Root::context` value beside
//! `crate::services::Services`, read by [`crate::hooks::use_settings`] and
//! [`crate::hooks::use_section`].

use sill_settings::ShellFile;
use tokio::sync::watch;

/// Every settled `settings.toml`. A watch receiver, never a Dioxus signal.
#[derive(Debug, Clone)]
pub struct SettingsFeed(pub watch::Receiver<ShellFile>);

impl SettingsFeed {
    /// A feed that never changes, for a headless render or a test.
    pub fn fixed(file: ShellFile) -> SettingsFeed {
        let (_tx, rx) = watch::channel(file);
        SettingsFeed(rx)
    }
}
