//! `use_settings`: `sill/settings.toml`, live.

use dioxus::prelude::{ReadSignal, use_context};
use shell_host::use_watch;
use sill_settings::{SettingsFeed, ShellFile};

/// Every settled `settings.toml`; a surface reads the domains it draws.
pub fn use_settings() -> ReadSignal<ShellFile> {
    use_watch(use_context::<SettingsFeed>().0)
}
