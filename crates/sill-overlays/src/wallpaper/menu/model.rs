//! The desktop menu as a [`MenuModel`]: what the entries are, as a function of what the menu
//! was opened over ([`MenuWorld`]).

use ds::prelude::*;
use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_ui_kit::menu::MenuModel;

/// The apps that pick a wallpaper, best first: Settings on its Wallpaper page, then on its
/// first page. The menu offers Change Wallpaper only when the app index has one.
pub(super) const PICKERS: [&str; 2] = ["org.quire.Settings.Wallpaper", "org.quire.Settings"];

/// One entry of the desktop menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::wallpaper) enum DesktopPick {
    ChangeWallpaper,
}

/// What the menu is opened over: the wallpaper picker the machine has, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::wallpaper) struct MenuWorld {
    pub(super) picker: Option<AppId>,
}

impl MenuWorld {
    /// The world over `apps`.
    pub(in crate::wallpaper) fn of(apps: &AppIndex) -> MenuWorld {
        MenuWorld {
            picker: picker_in(apps),
        }
    }
}

/// The first of [`PICKERS`] that `apps` has.
pub(super) fn picker_in(apps: &AppIndex) -> Option<AppId> {
    PICKERS
        .iter()
        .map(|id| AppId((*id).to_owned()))
        .find(|id| apps.get(id).is_some())
}

impl MenuModel for MenuWorld {
    type Pick = DesktopPick;

    fn entries(&self) -> Vec<MenuItem<DesktopPick>> {
        entries(self)
    }
}

/// The entries, top to bottom: none without a picker.
pub(super) fn entries(world: &MenuWorld) -> Vec<MenuItem<DesktopPick>> {
    world
        .picker
        .iter()
        .map(|_| MenuItem::new(DesktopPick::ChangeWallpaper, "Change Wallpaper\u{2026}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use sill_launcher::AppId;
    use sill_model::apps::AppIndex;
    use sill_ui_kit::harness::fake::dock::{SETTINGS, index};

    use super::{DesktopPick, MenuWorld, PICKERS, entries, picker_in};

    fn titles(items: &[MenuItem<DesktopPick>]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                MenuItem::Item { title, .. } | MenuItem::Info { title, .. } => title.clone(),
                MenuItem::Header(title) => format!("# {title}"),
                MenuItem::Separator => "-".to_owned(),
                MenuItem::Submenu { title, .. } => format!("{title} [..]"),
            })
            .collect()
    }

    #[test]
    fn the_menu_offers_change_wallpaper_only_with_a_picker() {
        let with = MenuWorld {
            picker: Some(AppId(SETTINGS.to_owned())),
        };
        assert_eq!(titles(&entries(&with)), ["Change Wallpaper\u{2026}"]);
        let without = MenuWorld { picker: None };
        assert!(entries(&without).is_empty(), "no picker, no entry");
    }

    #[test]
    fn the_wallpaper_picker_is_the_best_app_the_machine_has() {
        let settings = index();
        assert_eq!(
            picker_in(&settings),
            Some(AppId(PICKERS[1].to_owned())),
            "the settings app alone"
        );
        let mut both = settings.clone();
        let mut page = both.apps[both
            .apps
            .iter()
            .position(|a| a.id.0 == SETTINGS)
            .unwrap_or(0)]
        .clone();
        page.id = AppId(PICKERS[0].to_owned());
        both.apps.push(page);
        both.apps.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(picker_in(&both), Some(AppId(PICKERS[0].to_owned())));
        assert_eq!(picker_in(&AppIndex::default()), None);
    }
}
