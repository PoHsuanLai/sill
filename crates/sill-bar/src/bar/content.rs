//! What each bar menu lists, pure over the service values: a ds `Menu{Dropdown}`'s entries, the
//! control center, or a control center module (design/20 §1.1 and §1.5; the app name "its
//! windows and Quit"; the Sound, Wi-Fi and Battery items' dropdowns are their modules). The
//! clock opens no menu: it toggles the notification center.
//!
//! Status lines are `MenuEntry::Info` rows: an item's weight without the header's eyebrow,
//! never a choice (quire's bar gaps).

use ds::{Check, MenuEntry, MenuTrail};
use sill_launcher::WindowKey;
use sill_model::apps::{AppIndex, match_app};
use sill_model::cosmic_wl::Toplevels;
use sill_model::ids::OutputName;

use sill_settings::{ControlCenterModule, ControlCenterSettings};

use super::menus::BarMenu;
use super::pick::BarPick;

/// The frontmost app on an output: its name and every one of its windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrontApp {
    pub name: String,
    pub windows: Vec<(WindowKey, String)>,
}

/// What a bar popup draws.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PopupBody {
    Menu(Vec<MenuEntry<BarPick>>),
    /// The control center (`surfaces::control_center::ControlCenter`), which reads its own
    /// hooks; the body carries only what sizes the popup.
    ControlCenter(ControlCenterSettings),
    /// One module's dropdown from its own bar item: the module's control center component
    /// (`control_center::modules::Module`), reading its own hooks.
    Module(ControlCenterModule),
}

/// Everything the menus are built from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarState {
    pub app: Option<FrontApp>,
    /// `control_center.*`, which sizes its popup.
    pub control_center: ControlCenterSettings,
}

/// `menu`'s contents in `state`.
pub(crate) fn body(menu: BarMenu, state: &BarState) -> PopupBody {
    match menu {
        BarMenu::App => PopupBody::Menu(app_menu(state.app.as_ref())),
        BarMenu::ControlCenter => PopupBody::ControlCenter(state.control_center.clone()),
        BarMenu::NowPlaying
        | BarMenu::Display
        | BarMenu::Volume
        | BarMenu::Bluetooth
        | BarMenu::Network
        | BarMenu::Battery => menu
            .module()
            .map_or_else(|| PopupBody::Menu(Vec::new()), PopupBody::Module),
    }
}

/// The frontmost app on `output` (any output when the bar's has no name): the most recently
/// activated window there, named through the app index when it matches an entry.
pub(crate) fn front_app(
    toplevels: &Toplevels,
    apps: &AppIndex,
    output: Option<&OutputName>,
) -> Option<FrontApp> {
    let front = toplevels
        .windows
        .iter()
        .find(|w| output.is_none_or(|o| w.outputs.contains(o)))?;
    let name = match_app(&front.app_id, None, apps)
        .and_then(|id| apps.get(&id).map(|app| app.name.clone()))
        .unwrap_or_else(|| front.app_id.clone());
    let windows = toplevels
        .windows
        .iter()
        .filter(|w| w.app_id == front.app_id)
        .map(|w| (w.key, w.title.clone()))
        .collect();
    Some(FrontApp { name, windows })
}

/// The app menu: a header, one item per window, Quit; then "Shut Down…", which opens the power
/// menu (design/20 §1.8; the bar has no system menu of its own).
fn app_menu(app: Option<&FrontApp>) -> Vec<MenuEntry<BarPick>> {
    let power = [
        MenuEntry::Separator,
        item("Shut Down…".to_owned(), BarPick::PowerMenu, None),
    ];
    let Some(app) = app else {
        return lines_menu(["Desktop".to_owned()])
            .into_iter()
            .chain(power)
            .collect();
    };
    let windows = app
        .windows
        .iter()
        .map(|(key, title)| item(title.clone(), BarPick::Activate(*key), None));
    let quit = item(
        format!("Quit {}", app.name),
        BarPick::Quit(app.windows.iter().map(|(key, _)| *key).collect()),
        None,
    );
    std::iter::once(MenuEntry::Header(app.name.clone()))
        .chain(windows)
        .chain([MenuEntry::Separator, quit])
        .chain(power)
        .collect()
}

/// Status lines, then Settings.
fn lines_menu<const N: usize>(lines: [String; N]) -> Vec<MenuEntry<BarPick>> {
    lines
        .into_iter()
        .map(|title| MenuEntry::Info {
            title,
            detail: None,
        })
        .chain([
            MenuEntry::Separator,
            item("Settings…".to_owned(), BarPick::Settings, None),
        ])
        .collect()
}

/// A plain item.
fn item(title: String, value: BarPick, check: Option<Check>) -> MenuEntry<BarPick> {
    MenuEntry::Item {
        value,
        title,
        detail: None,
        tile: None,
        trail: MenuTrail::None,
        check,
        availability: ds::Availability::Enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::{BarState, FrontApp, PopupBody, body, front_app};
    use crate::bar::menus::BarMenu;
    use crate::bar::pick::BarPick;
    use ds::MenuEntry;
    use sill_launcher::WindowKey;
    use sill_model::apps::AppIndex;
    use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState};
    use sill_model::ids::OutputName;
    use sill_settings::ControlCenterModule;

    fn window(key: u64, app: &str, output: &str) -> Toplevel {
        Toplevel {
            key: WindowKey(key),
            app_id: app.to_owned(),
            title: format!("{app} {key}"),
            activity: Activity::Inactive,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: vec![OutputName(output.to_owned())],
        }
    }

    fn titles(entries: &[MenuEntry<BarPick>]) -> Vec<String> {
        entries
            .iter()
            .map(|e| match e {
                MenuEntry::Header(t) => format!("# {t}"),
                MenuEntry::Item { title, .. } => title.clone(),
                MenuEntry::Submenu { title, .. } => format!("{title} >"),
                MenuEntry::Info { title, .. } => format!("i {title}"),
                MenuEntry::Row(row) => format!("r {}", row.title.plain_text()),
                MenuEntry::Separator => "--".to_owned(),
            })
            .collect()
    }

    #[test]
    fn the_front_app_is_the_most_recent_window_on_this_output() {
        let toplevels = Toplevels {
            windows: vec![
                window(1, "foot", "DP-2"),
                window(2, "files", "DP-1"),
                window(3, "foot", "DP-1"),
            ],
        };
        let dp1 = OutputName("DP-1".to_owned());
        let app = front_app(&toplevels, &AppIndex::default(), Some(&dp1));
        assert_eq!(
            app,
            Some(FrontApp {
                name: "files".to_owned(),
                windows: vec![(WindowKey(2), "files 2".to_owned())],
            })
        );
        let any = front_app(&toplevels, &AppIndex::default(), None).map(|a| a.windows.len());
        assert_eq!(any, Some(2), "foot's two windows, on any output");
        assert_eq!(
            front_app(&Toplevels::default(), &AppIndex::default(), None),
            None
        );
    }

    #[test]
    fn each_menu_lists_its_own_lines() {
        let state = BarState {
            app: Some(FrontApp {
                name: "Foot".to_owned(),
                windows: vec![(WindowKey(4), "~".to_owned())],
            }),
            control_center: sill_settings::ControlCenterSettings::default(),
        };
        let menu = |m| match body(m, &state) {
            PopupBody::Menu(entries) => titles(&entries),
            PopupBody::ControlCenter(_) => vec!["control center".to_owned()],
            PopupBody::Module(module) => vec![format!("{module:?}")],
        };
        assert_eq!(
            menu(BarMenu::App),
            ["# Foot", "~", "--", "Quit Foot", "--", "Shut Down…"]
        );
        assert_eq!(menu(BarMenu::ControlCenter), ["control center"]);
        assert_eq!(menu(BarMenu::Bluetooth), ["Bluetooth"]);
        // The status items drop down their control center modules.
        for (item, module) in [
            (BarMenu::Volume, ControlCenterModule::Sound),
            (BarMenu::Network, ControlCenterModule::Wifi),
            (BarMenu::Battery, ControlCenterModule::Battery),
            (BarMenu::Display, ControlCenterModule::Display),
            (BarMenu::NowPlaying, ControlCenterModule::NowPlaying),
        ] {
            assert_eq!(body(item, &state), PopupBody::Module(module), "{item:?}");
        }
        let picks: Vec<BarPick> = match body(BarMenu::App, &state) {
            PopupBody::Menu(entries) => entries
                .into_iter()
                .filter_map(|entry| match entry {
                    MenuEntry::Item { value, .. } => Some(value),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        assert_eq!(
            picks[1..],
            [BarPick::Quit(vec![WindowKey(4)]), BarPick::PowerMenu]
        );
    }
}
