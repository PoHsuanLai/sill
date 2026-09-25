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
    Display,
    Osd,
    Window,
    PowerMenu,
    Session,
    Widgets,
    Calendar,
    Screenshot,
    HotCorners,
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
        (old.display != new.display, SettingsChange::Display),
        (old.osd != new.osd, SettingsChange::Osd),
        (old.window != new.window, SettingsChange::Window),
        (old.power_menu != new.power_menu, SettingsChange::PowerMenu),
        (old.session != new.session, SettingsChange::Session),
        (old.widgets != new.widgets, SettingsChange::Widgets),
        (old.calendar != new.calendar, SettingsChange::Calendar),
        (old.screenshot != new.screenshot, SettingsChange::Screenshot),
        (
            old.hot_corners != new.hot_corners,
            SettingsChange::HotCorners,
        ),
    ]
    .into_iter()
    .filter_map(|(changed, domain)| changed.then_some(domain))
    .collect()
}
