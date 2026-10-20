//! The bar's menus: one per title and status item (design/20 §1.1), each a [`MenuId`] for the
//! tracking machine and a [`MenuSide`] for the popup configuration.

use ds::base::word::Word;
use sill_settings::ControlCenterModule;

use crate::control_center::modules::module;

use super::popup::MenuSide;
use super::track::MenuId;

/// Which of the tray's items a menu belongs to: its place in the tray's registration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TraySlot(pub u16);

/// The machine's id for the first tray item; the bar's own menus are numbered below it.
const TRAY_ID_BASE: u32 = 1000;

/// Which of the focused app's own menus (File, Edit, ...) a title is: its place after the app
/// name, the app's first menu being the app name's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AppTitle(pub u16);

/// The machine's id for the first of the app's titles; the tray's ids are below it.
const APP_TITLE_ID_BASE: u32 = 200_000;

/// A bar title or status item that opens a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BarMenu {
    /// The system menu, always first (macOS's Apple menu): System Settings and the session's
    /// power actions, whichever app is in front.
    System,
    /// The frontmost app's name: its windows and Quit.
    App,
    /// Now Playing's own item (`control_center.menu_bar_now_playing`).
    NowPlaying,
    /// Display's own item (`control_center.menu_bar_display`).
    Display,
    /// Sound's item (`control_center.menu_bar_sound`).
    Volume,
    /// Bluetooth's own item (`control_center.menu_bar_bluetooth`).
    Bluetooth,
    /// Wi-Fi's item (`control_center.menu_bar_wifi`).
    Network,
    /// Battery's item (`control_center.menu_bar_battery`).
    Battery,
    /// The control center (design/20 §1.5): the last status item, left of the clock.
    ControlCenter,
    /// One of the focused app's menus after its name: File, Edit, View, Window, Help.
    AppMenu(AppTitle),
    /// A tray item's dbusmenu (the tray sits left of the modules, so it is not in [`BarMenu::ALL`]).
    Tray(TraySlot),
}

impl BarMenu {
    /// Every menu the bar always has, left to right along it. A tray item's menu comes and goes
    /// with the item.
    pub(crate) const ALL: [BarMenu; 9] = [
        BarMenu::System,
        BarMenu::App,
        BarMenu::NowPlaying,
        BarMenu::Display,
        BarMenu::Volume,
        BarMenu::Bluetooth,
        BarMenu::Network,
        BarMenu::Battery,
        BarMenu::ControlCenter,
    ];

    /// The machine's name for this menu.
    pub(crate) fn id(self) -> MenuId {
        MenuId(match self {
            BarMenu::App => 0,
            BarMenu::Volume => 1,
            BarMenu::Network => 2,
            BarMenu::Battery => 3,
            BarMenu::ControlCenter => 5,
            BarMenu::NowPlaying => 6,
            BarMenu::Display => 7,
            BarMenu::Bluetooth => 8,
            BarMenu::System => 9,
            BarMenu::Tray(TraySlot(slot)) => TRAY_ID_BASE + u32::from(slot),
            BarMenu::AppMenu(AppTitle(index)) => APP_TITLE_ID_BASE + u32::from(index),
        })
    }

    /// The menu the machine names `id`, if it is one of the bar's.
    pub(crate) fn of(id: MenuId) -> Option<BarMenu> {
        let title =
            id.0.checked_sub(APP_TITLE_ID_BASE)
                .and_then(|index| u16::try_from(index).ok())
                .map(|index| BarMenu::AppMenu(AppTitle(index)));
        let tray =
            id.0.checked_sub(TRAY_ID_BASE)
                .and_then(|slot| u16::try_from(slot).ok())
                .map(|slot| BarMenu::Tray(TraySlot(slot)));
        title
            .or(tray)
            .or_else(|| BarMenu::ALL.into_iter().find(|menu| menu.id() == id))
    }

    /// Which side of its title the menu hangs from (design/13 §13.3.2).
    pub(crate) fn side(self) -> MenuSide {
        match self {
            BarMenu::System | BarMenu::App | BarMenu::AppMenu(_) => MenuSide::Title,
            BarMenu::ControlCenter => MenuSide::OutputEdge,
            BarMenu::NowPlaying
            | BarMenu::Display
            | BarMenu::Volume
            | BarMenu::Bluetooth
            | BarMenu::Network
            | BarMenu::Battery
            | BarMenu::Tray(_) => MenuSide::Status,
        }
    }

    /// The item's accessible name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            BarMenu::System => "System",
            BarMenu::App => "Application",
            BarMenu::AppMenu(_) => "Application menu",
            BarMenu::Volume => "Volume",
            BarMenu::Network => "Network",
            BarMenu::Battery => "Battery",
            BarMenu::ControlCenter => "Control Center",
            BarMenu::NowPlaying => "Now Playing",
            BarMenu::Display => "Display",
            BarMenu::Bluetooth => "Bluetooth",
            BarMenu::Tray(_) => "Status item",
        }
    }

    /// The control center module whose component a module item's dropdown shows; `None` for the
    /// system and app menus and the control center.
    pub(crate) fn module(self) -> Option<ControlCenterModule> {
        ControlCenterModule::ALL.iter().copied().find(|&kind| {
            module(kind)
                .bar_item()
                .is_some_and(|item| item.menu == self)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AppTitle, BarMenu, TraySlot};
    use crate::bar::popup::MenuSide;

    #[test]
    fn every_menu_round_trips_through_its_id_and_hangs_from_its_side() {
        for menu in BarMenu::ALL {
            assert_eq!(BarMenu::of(menu.id()), Some(menu), "{menu:?}");
            let side = match menu {
                BarMenu::System | BarMenu::App => MenuSide::Title,
                BarMenu::ControlCenter => MenuSide::OutputEdge,
                _ => MenuSide::Status,
            };
            assert_eq!(menu.side(), side, "{menu:?}");
        }
        assert_eq!(BarMenu::of(crate::bar::track::MenuId(99)), None);
        for index in [0, 1, 9, u16::MAX] {
            let menu = BarMenu::AppMenu(AppTitle(index));
            assert_eq!(BarMenu::of(menu.id()), Some(menu), "title {index}");
            assert_eq!(menu.side(), MenuSide::Title, "title {index}");
            assert_eq!(menu.module(), None, "title {index}");
        }
        for slot in [0, 1, 7, u16::MAX] {
            let menu = BarMenu::Tray(TraySlot(slot));
            assert_eq!(BarMenu::of(menu.id()), Some(menu), "tray {slot}");
            assert_eq!(menu.side(), MenuSide::Status, "tray {slot}");
            assert_eq!(menu.module(), None, "tray {slot}");
        }
        let modules: Vec<_> = BarMenu::ALL
            .into_iter()
            .filter_map(BarMenu::module)
            .collect();
        assert_eq!(
            modules,
            crate::control_center::MENU_BAR_ORDER,
            "the module items stand in the bar in the order they are offered"
        );
    }
}
