//! The bar's menus: one per title and status item (design/20 §1.1), each a [`MenuId`] for the
//! tracking machine and a [`MenuSide`] for the popup configuration.

use sill_settings::ControlCenterModule;

use super::menu_track::MenuId;
use super::popup::MenuSide;

/// A bar title or status item that opens a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BarMenu {
    /// The frontmost app's name: its windows and Quit.
    App,
    /// Now Playing's own item (`control_center.menu_bar_now_playing`, M5).
    NowPlaying,
    /// Display's own item (`control_center.menu_bar_display`, M5).
    Display,
    /// Sound's item (`control_center.menu_bar_sound`).
    Volume,
    /// Bluetooth's own item (`control_center.menu_bar_bluetooth`, M5).
    Bluetooth,
    /// Wi-Fi's item (`control_center.menu_bar_wifi`).
    Network,
    /// Battery's item (`control_center.menu_bar_battery`).
    Battery,
    /// The control center (design/20 §1.5): the last status item, left of the clock (M5).
    ControlCenter,
    Clock,
}

impl BarMenu {
    /// Every menu, left to right along the bar.
    pub(crate) const ALL: [BarMenu; 9] = [
        BarMenu::App,
        BarMenu::NowPlaying,
        BarMenu::Display,
        BarMenu::Volume,
        BarMenu::Bluetooth,
        BarMenu::Network,
        BarMenu::Battery,
        BarMenu::ControlCenter,
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
            BarMenu::ControlCenter => 5,
            BarMenu::NowPlaying => 6,
            BarMenu::Display => 7,
            BarMenu::Bluetooth => 8,
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
            BarMenu::ControlCenter => MenuSide::OutputEdge,
            BarMenu::NowPlaying
            | BarMenu::Display
            | BarMenu::Volume
            | BarMenu::Bluetooth
            | BarMenu::Network
            | BarMenu::Battery
            | BarMenu::Clock => MenuSide::Status,
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
            BarMenu::ControlCenter => "Control Center",
            BarMenu::NowPlaying => "Now Playing",
            BarMenu::Display => "Display",
            BarMenu::Bluetooth => "Bluetooth",
        }
    }

    /// The control center module whose component a module item's dropdown shows (M5: the
    /// volume, network and battery menus become the Sound, Wi-Fi and Battery modules); `None`
    /// for the app menu, the control center and the clock.
    pub(crate) fn module(self) -> Option<ControlCenterModule> {
        match self {
            BarMenu::NowPlaying => Some(ControlCenterModule::NowPlaying),
            BarMenu::Display => Some(ControlCenterModule::Display),
            BarMenu::Volume => Some(ControlCenterModule::Sound),
            BarMenu::Bluetooth => Some(ControlCenterModule::Bluetooth),
            BarMenu::Network => Some(ControlCenterModule::Wifi),
            BarMenu::Battery => Some(ControlCenterModule::Battery),
            BarMenu::App | BarMenu::ControlCenter | BarMenu::Clock => None,
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
            let side = match menu {
                BarMenu::App => MenuSide::Title,
                BarMenu::ControlCenter => MenuSide::OutputEdge,
                _ => MenuSide::Status,
            };
            assert_eq!(menu.side(), side, "{menu:?}");
        }
        assert_eq!(
            BarMenu::of(crate::surfaces::bar::menu_track::MenuId(99)),
            None
        );
        let modules: Vec<_> = BarMenu::ALL
            .into_iter()
            .filter_map(BarMenu::module)
            .collect();
        assert_eq!(
            modules,
            crate::surfaces::control_center::MENU_BAR_ORDER,
            "the module items stand in the bar in the order they are offered"
        );
    }
}
