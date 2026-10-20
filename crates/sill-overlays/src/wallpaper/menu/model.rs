//! The desktop menu as a [`MenuModel`]: Change Wallpaper..., always.

use ds::prelude::*;
use sill_ui_kit::menu::MenuModel;

/// One entry of the desktop menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::wallpaper) enum DesktopPick {
    ChangeWallpaper,
}

/// The desktop's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::wallpaper) struct DesktopMenu;

impl MenuModel for DesktopMenu {
    type Pick = DesktopPick;

    fn entries(&self) -> Vec<MenuItem<DesktopPick>> {
        vec![MenuItem::new(
            DesktopPick::ChangeWallpaper,
            "Change Wallpaper\u{2026}",
        )]
    }
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use sill_ui_kit::menu::MenuModel;

    use super::{DesktopMenu, DesktopPick};

    #[test]
    fn the_menu_offers_change_wallpaper() {
        let entries = DesktopMenu.entries();
        assert!(matches!(
            entries.as_slice(),
            [MenuItem::Item {
                value: DesktopPick::ChangeWallpaper,
                title,
                ..
            }] if title == "Change Wallpaper\u{2026}"
        ));
    }
}
