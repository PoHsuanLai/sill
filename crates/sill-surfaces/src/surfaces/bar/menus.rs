//! The bar's menus: one per title and status item (design/20 §1.1), each a [`MenuId`] for the
//! tracking machine and a [`MenuSide`] for the popup configuration.

use super::menu_track::MenuId;
use super::popup::MenuSide;

/// A bar title or status item that opens a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BarMenu {
    /// The frontmost app's name: its windows and Quit.
    App,
    Volume,
    Network,
    Battery,
    Clock,
}

impl BarMenu {
    /// Every menu, left to right along the bar.
    pub(crate) const ALL: [BarMenu; 5] = [
        BarMenu::App,
        BarMenu::Volume,
        BarMenu::Network,
        BarMenu::Battery,
        BarMenu::Clock,
    ];

    /// The machine's name for this menu.
    pub(crate) fn id(self) -> MenuId {
        MenuId(match self {
            BarMenu::App => 0,
            BarMenu::Volume => 1,
            BarMenu::Network => 2,
            BarMenu::Battery => 3,
            BarMenu::Clock => 4,
        })
    }

    /// The menu the machine names `id`, if it is one of the bar's.
    pub(crate) fn of(id: MenuId) -> Option<BarMenu> {
        BarMenu::ALL.into_iter().find(|menu| menu.id() == id)
    }

    /// Which side of its title the menu hangs from (design/13 §13.3.2).
    pub(crate) fn side(self) -> MenuSide {
        match self {
            BarMenu::App => MenuSide::Title,
            BarMenu::Volume | BarMenu::Network | BarMenu::Battery | BarMenu::Clock => {
                MenuSide::Status
            }
        }
    }

    /// The item's accessible name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            BarMenu::App => "Application",
            BarMenu::Volume => "Volume",
            BarMenu::Network => "Network",
            BarMenu::Battery => "Battery",
            BarMenu::Clock => "Clock",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BarMenu;
    use crate::surfaces::bar::popup::MenuSide;

    #[test]
    fn every_menu_round_trips_through_its_id_and_only_the_app_hangs_right() {
        for menu in BarMenu::ALL {
            assert_eq!(BarMenu::of(menu.id()), Some(menu), "{menu:?}");
            let side = if menu == BarMenu::App {
                MenuSide::Title
            } else {
                MenuSide::Status
            };
            assert_eq!(menu.side(), side, "{menu:?}");
        }
        assert_eq!(
            BarMenu::of(crate::surfaces::bar::menu_track::MenuId(99)),
            None
        );
    }
}
