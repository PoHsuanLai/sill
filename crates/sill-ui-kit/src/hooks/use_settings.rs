//! `use_settings` and `use_section`: `sill/settings.toml`, live.

use dioxus::prelude::{ReadSignal, ReadableExt, use_context, use_memo};
use shell_host::use_watch;
use sill_settings::{Section, ShellFile};

use crate::settings::SettingsFeed;

/// Every settled `settings.toml`; a surface reads the domains it draws.
pub fn use_settings() -> ReadSignal<ShellFile> {
    use_watch(use_context::<SettingsFeed>().0)
}

/// One domain of the settings: `use_section::<BarSettings>()`. It changes only when that domain
/// does, so a surface that draws one table does not redraw for another's edit.
pub fn use_section<S: Section>() -> ReadSignal<S> {
    let file = use_settings();
    ReadSignal::new(use_memo(move || S::of(&file.read()).clone()))
}
