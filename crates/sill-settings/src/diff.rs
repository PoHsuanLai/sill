//! Which domains changed between two reads (design/22 §4.4 `apply`, the sill half): a surface
//! repaints only for the domains it draws. Whole-struct comparison per domain, as §4.4 argues.

use crate::file::ShellFile;

/// One variant per sill domain, in table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsChange {
    Bar,
    Dock,
    Launcher,
    Scroll,
    Scrollbar,
    Menus,
    Switcher,
    Sound,
    Notifications,
    ControlCenter,
    Spaces,
}

/// The domains that differ between `old` and `new`, in table order. `version` and unknown
/// tables are not domains.
pub fn apply(old: &ShellFile, new: &ShellFile) -> Vec<SettingsChange> {
    [
        (old.bar != new.bar, SettingsChange::Bar),
        (old.dock != new.dock, SettingsChange::Dock),
        (old.launcher != new.launcher, SettingsChange::Launcher),
        (old.scroll != new.scroll, SettingsChange::Scroll),
        (old.scrollbar != new.scrollbar, SettingsChange::Scrollbar),
        (old.menus != new.menus, SettingsChange::Menus),
        (old.switcher != new.switcher, SettingsChange::Switcher),
        (old.sound != new.sound, SettingsChange::Sound),
        (
            old.notifications != new.notifications,
            SettingsChange::Notifications,
        ),
        (
            old.control_center != new.control_center,
            SettingsChange::ControlCenter,
        ),
        (old.spaces != new.spaces, SettingsChange::Spaces),
    ]
    .into_iter()
    .filter_map(|(changed, domain)| changed.then_some(domain))
    .collect()
}
