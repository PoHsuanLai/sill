//! The settings as a surface receives them: one more `Root::context` value beside
//! `sill_services::Services`, read by `sill-ui-kit::hooks::use_settings`.

use tokio::sync::watch;

use crate::file::ShellFile;

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
