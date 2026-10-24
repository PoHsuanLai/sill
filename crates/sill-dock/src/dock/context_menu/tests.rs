//! Table tests for the app tile's menu: its rows (`app.rs`) and its entries (`entries.rs`).

use super::{Checked, DockMenuRow, Minimized, OptionHeld, app, menu_entries, menu_windows};
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{TileApp, TileId};
use crate::dock::pick::{DockPick, LoginChange};
use ds::prelude::*;
use sill_launcher::{AppId, WindowKey};
use sill_model::apps::{App, AppIcon, DbusActivation, DesktopActionEntry, Listing, Terminal};
use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState};
use sill_settings::DockMenuOrder;
use std::path::PathBuf;

const TERM: &str = "foot";

fn term() -> App {
    App {
        id: AppId(TERM.to_owned()),
        name: "Example Terminal".to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: "foot".to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        bus_names: Vec::new(),
        intents_app: None,
        renamed_from: Vec::new(),
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: vec![DesktopActionEntry {
            id: "new-window".to_owned(),
            name: "New Window".to_owned(),
            exec: "foot".to_owned(),
        }],
        mime_types: Vec::new(),
        categories: Vec::new(),
        keyboard_profile: None,
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from("/usr/share/applications/foot.desktop"),
    }
}

fn window(key: u64, app_id: &str, activity: Activity, state: WindowState) -> Toplevel {
    Toplevel {
        identifier: String::new(),
        key: WindowKey(key),
        app_id: app_id.to_owned(),
        title: format!("shell {key}"),
        activity,
        window: state,
        sizing: Sizing::Floating,
        workspaces: Vec::new(),
        outputs: Vec::new(),
    }
}

fn toplevels() -> Toplevels {
    Toplevels {
        windows: vec![
            window(2, TERM, Activity::Active, WindowState::Normal),
            window(9, "firefox", Activity::Inactive, WindowState::Normal),
            window(1, TERM, Activity::Inactive, WindowState::Minimized),
        ],
    }
}

fn pinned(app: TileApp) -> DockItem {
    DockItem {
        tile: TileId(1),
        kind: ItemKind::Pinned(AppId(TERM.to_owned())),
        app,
    }
}

fn names(entries: &[MenuItem<DockPick>]) -> Vec<String> {
    entries
        .iter()
        .map(|e| match e {
            MenuItem::Header(t) => format!("# {t}"),
            MenuItem::Item { title, check, .. } => match check {
                Some(Check::On) => format!("{title} ✓"),
                _ => title.clone(),
            },
            MenuItem::Submenu {
                title, children, ..
            } => {
                format!("{title} > [{}]", names(children).join(", "))
            }
            MenuItem::Info { title, .. } => format!("i {title}"),
            MenuItem::Separator => "--".to_owned(),
            MenuItem::Swatches(row) => format!("* {}", row.title),
        })
        .collect()
}

fn menu(item: &DockItem, order: DockMenuOrder, option: OptionHeld) -> Vec<MenuItem<DockPick>> {
    menu_at_login(item, order, option, Checked::Unchecked)
}

fn menu_at_login(
    item: &DockItem,
    order: DockMenuOrder,
    option: OptionHeld,
    login: Checked,
) -> Vec<MenuItem<DockPick>> {
    let entry = term();
    let all = toplevels();
    let rows = app::rows(item, Some(&entry), &all, order, option, login);
    let windows = menu_windows(item, Some(&entry), &all);
    menu_entries(&rows, item.kind.app_id(), &windows, order)
}

#[test]
fn a_running_pinned_app_lists_windows_actions_remove_and_quit() {
    let item = pinned(TileApp::Active { windows: 2 });
    let rows = app::rows(
        &item,
        Some(&term()),
        &toplevels(),
        DockMenuOrder::WindowsFirst,
        OptionHeld::Up,
        Checked::Unchecked,
    );
    assert_eq!(
        rows,
        vec![
            DockMenuRow::Window {
                title: "shell 2".to_owned(),
                active: Checked::Checked,
                minimized: Minimized::Normal,
            },
            DockMenuRow::Window {
                title: "shell 1".to_owned(),
                active: Checked::Unchecked,
                minimized: Minimized::Minimized,
            },
            DockMenuRow::Separator,
            DockMenuRow::DesktopAction {
                id: "new-window".to_owned(),
                name: "New Window".to_owned(),
            },
            DockMenuRow::Separator,
            DockMenuRow::KeepInDock(Checked::Checked),
            DockMenuRow::Separator,
            DockMenuRow::Quit,
        ]
    );
    let entries = menu(&item, DockMenuOrder::WindowsFirst, OptionHeld::Up);
    assert_eq!(
        names(&entries),
        [
            "shell 2 ✓",
            "◆ shell 1",
            "--",
            "New Window",
            "--",
            "Remove from Dock",
            "--",
            "Quit"
        ]
    );
    let picks: Vec<DockPick> = entries
        .into_iter()
        .filter_map(|e| match e {
            MenuItem::Item { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(
        picks,
        vec![
            DockPick::Activate(WindowKey(2)),
            DockPick::Activate(WindowKey(1)),
            DockPick::Action {
                app: AppId(TERM.to_owned()),
                action: "new-window".to_owned()
            },
            DockPick::Unpin(AppId(TERM.to_owned())),
            DockPick::Quit(vec![WindowKey(2), WindowKey(1)]),
        ]
    );
}

#[test]
fn not_running_offers_no_quit_and_a_running_app_can_be_kept() {
    let idle = pinned(TileApp::NotRunning);
    let none = Toplevels::default();
    let rows = app::rows(
        &idle,
        Some(&term()),
        &none,
        DockMenuOrder::WindowsFirst,
        OptionHeld::Up,
        Checked::Unchecked,
    );
    assert_eq!(
        rows.last(),
        Some(&DockMenuRow::KeepInDock(Checked::Checked))
    );
    assert!(!rows.contains(&DockMenuRow::Quit));
    let running = DockItem {
        kind: ItemKind::Running(AppId(TERM.to_owned())),
        ..pinned(TileApp::Inactive)
    };
    let entries = menu(&running, DockMenuOrder::WindowsFirst, OptionHeld::Up);
    assert!(names(&entries).contains(&"Keep in Dock".to_owned()));
}

#[test]
fn apple_order_puts_the_options_in_a_submenu_and_adds_show_all_windows() {
    let item = pinned(TileApp::Inactive);
    let entries = menu(&item, DockMenuOrder::AppleOrder, OptionHeld::Held);
    assert_eq!(
        names(&entries),
        [
            "shell 2 ✓",
            "◆ shell 1",
            "--",
            "New Window",
            "--",
            "Options > [Remove from Dock, Open at Login, Show in Files]",
            "--",
            "Show All Windows",
            "Hide",
            "Force Quit"
        ]
    );
    let running = DockItem {
        kind: ItemKind::Running(AppId(TERM.to_owned())),
        ..pinned(TileApp::Inactive)
    };
    let kept = names(&menu(&running, DockMenuOrder::AppleOrder, OptionHeld::Up));
    assert!(kept.contains(&"Options > [Keep in Dock, Open at Login, Show in Files]".to_owned()));
}

#[test]
fn open_at_login_is_checked_when_the_folder_says_so_and_a_pick_flips_it() {
    let item = pinned(TileApp::Inactive);
    let login_pick = |login| {
        let entries = menu_at_login(&item, DockMenuOrder::AppleOrder, OptionHeld::Up, login);
        let options = entries.into_iter().find_map(|e| match e {
            MenuItem::Submenu { children, .. } => Some(children),
            _ => None,
        });
        options
            .into_iter()
            .flatten()
            .find_map(|e| match e {
                MenuItem::Item {
                    title,
                    value,
                    check,
                    ..
                } if title == "Open at Login" => Some((check, value)),
                _ => None,
            })
            .expect("Open at Login is offered")
    };
    let app = || AppId(TERM.to_owned());
    assert_eq!(
        login_pick(Checked::Unchecked),
        (
            None,
            DockPick::OpenAtLogin(LoginChange::On {
                app: app(),
                entry: PathBuf::from("/usr/share/applications/foot.desktop"),
            })
        )
    );
    assert_eq!(
        login_pick(Checked::Checked),
        (
            Some(Check::On),
            DockPick::OpenAtLogin(LoginChange::Off { app: app() })
        )
    );
}

#[test]
fn show_all_windows_brings_back_the_minimized_ones_and_raises_the_front() {
    let item = pinned(TileApp::Inactive);
    let entries = menu(&item, DockMenuOrder::AppleOrder, OptionHeld::Up);
    let pick = entries.into_iter().find_map(|e| match e {
        MenuItem::Item { title, value, .. } if title == "Show All Windows" => Some(value),
        _ => None,
    });
    assert_eq!(
        pick,
        Some(DockPick::ShowAll {
            minimized: vec![WindowKey(1)],
            front: Some(WindowKey(2)),
        })
    );
}

#[test]
fn windows_first_has_neither_options_nor_show_all_windows() {
    let item = pinned(TileApp::Inactive);
    let listed = names(&menu(&item, DockMenuOrder::WindowsFirst, OptionHeld::Up));
    let extra = |n: &&String| n.starts_with("Options") || n.as_str() == "Show All Windows";
    assert!(!listed.iter().any(|n| extra(&n)));
    assert!(!listed.contains(&"Open at Login".to_owned()));
}
