//! What each bar menu lists, pure over the service values: a ds `Menu{Dropdown}`'s entries, the
//! control center, or a control center module (design/20 §1.1 and §1.5; the app name "its
//! windows and Quit"; the Sound, Wi-Fi and Battery items' dropdowns are their modules). The
//! clock opens no menu: it toggles the notification center.
//!
//! Status lines are `MenuItem::Info` rows: an item's weight without the header's eyebrow,
//! never a choice (quire's bar gaps).

use ds::prelude::*;
use sill_launcher::WindowKey;
use sill_model::app_menu::{AppMenus, MenuApp, OwnGroup, QuitRow, Titles, own_groups, titles};
use sill_model::apps::{AppIndex, match_app};
use sill_model::ids::OutputName;
use sill_model::tray::Tray;
use sill_model::windows::Toplevels;

use sill_settings::{ControlCenterModule, ControlCenterSettings};

use super::dbusmenu::{RowIcon, menu_items};
use super::menus::{AppTitle, BarMenu};
use super::pick::BarPick;

/// The frontmost app on an output: its name and every one of its windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrontApp {
    /// The Wayland `app_id`, which names the menu the app serves.
    pub app_id: String,
    pub name: String,
    pub windows: Vec<(WindowKey, String)>,
}

/// The focused app's exported menus: whose they are and the titles they make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppMenuState {
    pub app: MenuApp,
    pub titles: Titles,
}

impl AppMenuState {
    /// The menus `app` serves in `menus`, none when it serves none (design/27 §5.2 rule c).
    pub(crate) fn of(app: Option<&FrontApp>, menus: &AppMenus) -> Option<AppMenuState> {
        let app = MenuApp(app?.app_id.clone());
        let served = menus.served.get(&app)?;
        Some(AppMenuState {
            titles: titles(served),
            app,
        })
    }

    /// The pick of the row with dbusmenu id `id`.
    fn pick(&self, id: i32) -> BarPick {
        BarPick::AppMenu {
            app: self.app.clone(),
            id,
        }
    }
}

/// What a bar popup draws.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PopupBody {
    Menu(Vec<MenuItem<BarPick>>),
    /// The control center (`control_center::ControlCenter`), which reads its own
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
    /// The tray's items and their dbusmenus.
    pub tray: Tray,
    /// The focused app's own menus, when it serves any.
    pub menus: Option<AppMenuState>,
}

/// `menu`'s contents in `state`, a tray row's icon drawn as `icon` says.
pub(crate) fn body(menu: BarMenu, state: &BarState, icon: RowIcon) -> PopupBody {
    match menu {
        BarMenu::Tray(slot) => PopupBody::Menu(
            state
                .tray
                .items
                .get(usize::from(slot.0))
                .map(|item| {
                    menu_items(
                        &|entry| BarPick::Tray {
                            item: item.id.clone(),
                            entry,
                        },
                        &item.menu,
                        icon,
                    )
                })
                .unwrap_or_default(),
        ),
        BarMenu::App => PopupBody::Menu(app_menu(state.app.as_ref(), state.menus.as_ref(), icon)),
        BarMenu::AppMenu(AppTitle(index)) => PopupBody::Menu(
            state
                .menus
                .as_ref()
                .and_then(|menus| {
                    let rows = &menus.titles.menus.get(usize::from(index))?.rows;
                    Some(menu_items(&|id| menus.pick(id), rows, icon))
                })
                .unwrap_or_default(),
        ),
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
    Some(FrontApp {
        app_id: front.app_id.clone(),
        name,
        windows,
    })
}

/// The app menu: a header, the app's own first menu when it serves one (About, Settings), then
/// the rows the shell makes from the compositor (windows, Minimise, Hide, Quit); then "Shut
/// Down…", which opens the power menu (design/20 §1.8; the bar has no system menu of its own).
fn app_menu(
    app: Option<&FrontApp>,
    menus: Option<&AppMenuState>,
    icon: RowIcon,
) -> Vec<MenuItem<BarPick>> {
    let power = [
        MenuItem::Separator,
        item("Shut Down…".to_owned(), BarPick::PowerMenu, None),
    ];
    let Some(app) = app else {
        return lines_menu(["Desktop".to_owned()])
            .into_iter()
            .chain(power)
            .collect();
    };
    let own = menus
        .and_then(|menus| {
            let (_, rows) = menus.titles.app.as_ref()?;
            Some(menu_items(&|id| menus.pick(id), rows, icon))
        })
        .unwrap_or_default();
    let quit = QuitRow::of_titles(menus.map(|menus| &menus.titles));
    let groups = own_groups(&app.name, &app.windows, quit);
    let shell = groups.into_iter().map(|group| own_rows(app, group));
    let sections = std::iter::once(own)
        .chain(shell)
        .filter(|rows| !rows.is_empty());
    let mut entries = vec![MenuItem::Header(app.name.clone())];
    for (at, rows) in sections.enumerate() {
        if at > 0 {
            entries.push(MenuItem::Separator);
        }
        entries.extend(rows);
    }
    entries.extend(power);
    entries
}

/// `group`'s rows as menu items.
fn own_rows(app: &FrontApp, group: OwnGroup) -> Vec<MenuItem<BarPick>> {
    group
        .into_iter()
        .map(|row| {
            let pick = BarPick::Own {
                app_id: app.app_id.clone(),
                item: row.item,
            };
            item(row.label, pick, None)
        })
        .collect()
}

/// Status lines, then Settings.
fn lines_menu<const N: usize>(lines: [String; N]) -> Vec<MenuItem<BarPick>> {
    lines
        .into_iter()
        .map(|title| MenuItem::Info {
            title,
            detail: None,
        })
        .chain([
            MenuItem::Separator,
            item("Settings…".to_owned(), BarPick::Settings, None),
        ])
        .collect()
}

/// A plain item.
fn item(title: String, value: BarPick, check: Option<Check>) -> MenuItem<BarPick> {
    MenuItem::Item {
        value,
        title,
        image: None,
        key: None,
        check,
        availability: Availability::Enabled,
        hint: None,
        after: AfterPick::Close,
        text: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{AppMenuState, BarState, FrontApp, PopupBody, body, front_app};
    use crate::bar::menus::AppTitle;
    use crate::bar::menus::BarMenu;
    use crate::bar::menus::TraySlot;
    use crate::bar::pick::BarPick;
    use ds::prelude::*;
    use sill_launcher::WindowKey;
    use sill_model::app_menu::{AppMenus, MenuApp, MenuName, OwnItem, ServedMenu};
    use sill_model::apps::AppIndex;
    use sill_model::ids::OutputName;
    use sill_model::tray::{
        Check, Chord, Tray, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuIcon,
        TrayMenuKind, TrayStatus,
    };
    use sill_model::units::{Availability as Enabled, Visibility};
    use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState};
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

    fn titles(entries: &[MenuItem<BarPick>]) -> Vec<String> {
        entries
            .iter()
            .map(|e| match e {
                MenuItem::Header(t) => format!("# {t}"),
                MenuItem::Item { title, .. } => title.clone(),
                MenuItem::Submenu { title, .. } => format!("{title} >"),
                MenuItem::Info { title, .. } => format!("i {title}"),
                MenuItem::Separator => "--".to_owned(),
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
                app_id: "files".to_owned(),
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
                app_id: "foot".to_owned(),
                name: "Foot".to_owned(),
                windows: vec![(WindowKey(4), "~".to_owned())],
            }),
            control_center: sill_settings::ControlCenterSettings::default(),
            tray: Tray::default(),
            menus: None,
        };
        let menu = |m| match body(m, &state, &|_| None) {
            PopupBody::Menu(entries) => titles(&entries),
            PopupBody::ControlCenter(_) => vec!["control center".to_owned()],
            PopupBody::Module(module) => vec![format!("{module:?}")],
        };
        assert_eq!(
            menu(BarMenu::App),
            [
                "# Foot",
                "~",
                "--",
                "Minimise",
                "Hide Foot",
                "Hide Others",
                "Show All",
                "--",
                "Quit Foot",
                "--",
                "Shut Down…"
            ]
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
            assert_eq!(
                body(item, &state, &|_| None),
                PopupBody::Module(module),
                "{item:?}"
            );
        }
        let picks: Vec<BarPick> = match body(BarMenu::App, &state, &|_| None) {
            PopupBody::Menu(entries) => entries
                .into_iter()
                .filter_map(|entry| match entry {
                    MenuItem::Item { value, .. } => Some(value),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        let own = |item| BarPick::Own {
            app_id: "foot".to_owned(),
            item,
        };
        assert_eq!(
            picks[1..],
            [
                own(OwnItem::Minimise(WindowKey(4))),
                own(OwnItem::Hide),
                own(OwnItem::HideOthers),
                own(OwnItem::ShowAll),
                own(OwnItem::Quit),
                BarPick::PowerMenu,
            ]
        );
    }

    fn tray_item(name: &str, labels: &[&str]) -> TrayItem {
        let rows = labels.iter().zip(1..).map(|(label, id)| TrayMenuEntry {
            id,
            kind: TrayMenuKind::Item {
                label: (*label).to_owned(),
                enabled: Enabled::Enabled,
                visibility: Visibility::Shown,
                check: Check::None,
                icon: TrayMenuIcon::None,
                children: Vec::new(),
                shortcut: None,
                command: None,
            },
        });
        TrayItem {
            id: TrayItemId(format!(":1.{name}/StatusNotifierItem")),
            title: name.to_owned(),
            icon: TrayIcon::None,
            status: TrayStatus::Active,
            menu: rows.collect(),
        }
    }

    #[test]
    fn a_tray_slot_lists_its_own_items_dbusmenu() {
        let state = BarState {
            app: None,
            menus: None,
            control_center: sill_settings::ControlCenterSettings::default(),
            tray: Tray {
                items: vec![
                    tray_item("telegram", &["Open", "Quit"]),
                    tray_item("fcitx", &["Mozc"]),
                ],
            },
        };
        let menu = |slot| match body(BarMenu::Tray(TraySlot(slot)), &state, &|_| None) {
            PopupBody::Menu(entries) => titles(&entries),
            other => panic!("not a menu: {other:?}"),
        };
        assert_eq!(menu(0), ["Open", "Quit"]);
        assert_eq!(menu(1), ["Mozc"]);
        assert!(menu(2).is_empty(), "an item that has gone has no rows");
    }

    fn row(id: i32, label: &str, shortcut: Option<Chord>) -> TrayMenuEntry {
        TrayMenuEntry {
            id,
            kind: TrayMenuKind::Item {
                label: label.to_owned(),
                enabled: Enabled::Enabled,
                visibility: Visibility::Shown,
                check: Check::None,
                icon: TrayMenuIcon::None,
                children: Vec::new(),
                shortcut,
                command: None,
            },
        }
    }

    fn menu(id: i32, label: &str, rows: Vec<TrayMenuEntry>) -> TrayMenuEntry {
        let mut menu = row(id, label, None);
        if let TrayMenuKind::Item { children, .. } = &mut menu.kind {
            *children = rows;
        }
        menu
    }

    /// The focused app `foot` serving: its own menu (About, Quit), Edit (Undo with Cmd+Z), View.
    fn served_state() -> BarState {
        let cmd_z = Chord(vec!["Super".to_owned(), "Z".to_owned()]);
        let served = ServedMenu {
            name: MenuName("org.quire.AppMenu.foot".to_owned()),
            menus: vec![
                menu(
                    11,
                    "Foot",
                    vec![row(12, "About Foot", None), row(13, "Quit Foot", None)],
                ),
                menu(20, "Edit", vec![row(21, "Undo", Some(cmd_z))]),
                menu(40, "View", vec![row(41, "Sidebar", None)]),
            ],
        };
        let app = FrontApp {
            app_id: "foot".to_owned(),
            name: "Foot".to_owned(),
            windows: vec![(WindowKey(4), "~".to_owned())],
        };
        let menus = AppMenus {
            served: [(MenuApp("foot".to_owned()), served)].into(),
        };
        BarState {
            menus: AppMenuState::of(Some(&app), &menus),
            app: Some(app),
            control_center: sill_settings::ControlCenterSettings::default(),
            tray: Tray::default(),
        }
    }

    fn entries(menu: BarMenu, state: &BarState) -> Vec<MenuItem<BarPick>> {
        match body(menu, state, &|_| None) {
            PopupBody::Menu(entries) => entries,
            other => panic!("not a menu: {other:?}"),
        }
    }

    #[test]
    fn a_served_apps_first_menu_joins_the_app_name_and_the_rest_are_titles() {
        let state = served_state();
        // The app's own rows come before the shell's, and the shell offers no Quit of its own.
        assert_eq!(
            titles(&entries(BarMenu::App, &state)),
            [
                "# Foot",
                "About Foot",
                "Quit Foot",
                "--",
                "~",
                "--",
                "Minimise",
                "Hide Foot",
                "Hide Others",
                "Show All",
                "--",
                "Shut Down…"
            ]
        );
        let picks = |menu| -> Vec<BarPick> {
            entries(menu, &state)
                .into_iter()
                .filter_map(|entry| match entry {
                    MenuItem::Item { value, .. } => Some(value),
                    _ => None,
                })
                .collect()
        };
        let app = MenuApp("foot".to_owned());
        assert_eq!(
            picks(BarMenu::App)[..2],
            [
                BarPick::AppMenu {
                    app: app.clone(),
                    id: 12
                },
                BarPick::AppMenu {
                    app: app.clone(),
                    id: 13
                },
            ]
        );
        assert_eq!(
            titles(&entries(BarMenu::AppMenu(AppTitle(0)), &state)),
            ["Undo"]
        );
        assert_eq!(
            picks(BarMenu::AppMenu(AppTitle(1))),
            [BarPick::AppMenu {
                app: app.clone(),
                id: 41
            }]
        );
        assert!(entries(BarMenu::AppMenu(AppTitle(2)), &state).is_empty());
        let undo = entries(BarMenu::AppMenu(AppTitle(0)), &state);
        let key = match &undo[0] {
            MenuItem::Item { key, .. } => key.clone().map(|key| key.keys()),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            key,
            Some(vec![ShortcutKey::Super, ShortcutKey::Char('z')]),
            "Cmd+Z is the row's key equivalent"
        );
    }

    #[test]
    fn an_app_that_serves_nothing_keeps_the_app_menu_alone() {
        let mut state = served_state();
        state.menus = None;
        assert!(entries(BarMenu::AppMenu(AppTitle(0)), &state).is_empty());
        let app_menu = titles(&entries(BarMenu::App, &state));
        assert_eq!(app_menu.first().map(String::as_str), Some("# Foot"));
        assert!(app_menu.contains(&"Quit Foot".to_owned()), "{app_menu:?}");
    }
}
