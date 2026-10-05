//! The bar mounted as a `Surface` on the fake services: what the services hold is what the bar
//! shows, and a value fed after it started changes what it shows.

use ds_harness::Driver;
use std::time::Duration;

use sill_bar::Bar;
use sill_model::battery::Battery;
use sill_model::clock::ClockText;
use sill_model::cosmic_wl::Toplevels;
use sill_model::tray::{
    Check, Tray, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind,
    TrayStatus,
};
use sill_model::units::{Availability, Presence, Visibility};
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::ControlCenterLink;

/// The bar on the default settings, its first frame drawn.
fn start() -> SurfaceHarness<Bar> {
    let mut bar = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ()).start();
    bar.harness.advance(Duration::ZERO);
    bar
}

/// How many times `needle` occurs in `html`.
fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

#[test]
fn the_bar_shows_what_the_services_hold() {
    let html = start().harness.html();
    let cases = [
        ("the clock", "Thu 24 Sep 14:05", 1),
        ("the app name", ">foot<", 1),
        ("the volume item", "aria-label=\"Volume\"", 1),
        ("the network item", "aria-label=\"Network\"", 1),
        ("the battery item", "aria-label=\"Battery\"", 1),
        ("in one segmented group", "class=\"ds-ws-pills\"", 1),
        ("each a quire pill", "class=\"ds-ws-pill\"", 3),
        ("one current pill", "aria-current=\"true\"", 1),
        (
            "the app name and the clock as menu-bar title items",
            "data-image=\"leading\"",
            2,
        ),
        ("the app name bold", "data-emphasis=\"strong\"", 1),
        ("the shell type scale", "--shell-bar-font:13px;", 1),
        (
            "22 px menu rows for the popups",
            "--shell-menu-row-h:22px;",
            1,
        ),
        ("the material stack", "--m-vibrancy:1;", 1),
        ("the Bar material", "data-material=\"bar\"", 1),
        ("the root draws the Space", "data-frame=\"tinted\"", 1),
        ("on the frame ground", "data-ground=\"frame\"", 1),
        ("no tint layer of the bar's own", "sill-bar-tint", 0),
        // Sound, Wi-Fi and Battery (`control_center.menu_bar_*`), then the control center.
        ("four status items", "data-image=\"only\"", 4),
        (
            "the control center item",
            "aria-label=\"Control Center\"",
            1,
        ),
        (
            "their metrics",
            "--bar-status-box:22px;--bar-status-glyph:16px;",
            1,
        ),
        ("no menu open", "aria-expanded=\"true\"", 0),
    ];
    for (name, needle, want) in cases {
        assert_eq!(count(&html, needle), want, "{name}: {needle}\n{html}");
    }
}

#[test]
fn fed_values_replace_the_defaults() {
    let mut bar = start();
    let before = bar.harness.html();
    let feeds = &bar.feeds;
    feeds.clock.send_replace(ClockText {
        text: "Fri 25 Sep 09:00".to_owned(),
    });
    feeds.battery.send_replace(Battery {
        presence: Presence::Absent,
        ..Battery::default()
    });
    feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = Toplevels::default());
    bar.harness.advance(Duration::from_millis(40));
    let after = bar.harness.html();
    let cases = [
        ("the old time", "Thu 24 Sep 14:05", 1, 0),
        ("the new time", "Fri 25 Sep 09:00", 0, 1),
        ("the battery hides", "aria-label=\"Battery\"", 1, 0),
        ("no window: the desktop", ">Desktop<", 0, 1),
    ];
    for (name, needle, was, now) in cases {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}"
        );
    }
}

fn tray_item(name: &str, menu: Vec<TrayMenuEntry>) -> TrayItem {
    TrayItem {
        id: TrayItemId(format!(":1.{name}/StatusNotifierItem")),
        title: name.to_owned(),
        icon: TrayIcon::None,
        status: TrayStatus::Active,
        menu,
    }
}

#[test]
fn each_tray_item_is_a_status_item_and_none_has_a_menu_open() {
    let mut bar = start();
    let open = vec![TrayMenuEntry {
        id: 1,
        kind: TrayMenuKind::Item {
            label: "Open".to_owned(),
            enabled: Availability::Enabled,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children: Vec::new(),
            shortcut: None,
            command: None,
        },
    }];
    let before = bar.harness.html();
    bar.feeds.tray.send_replace(Tray {
        items: vec![tray_item("telegram", open), tray_item("fcitx", Vec::new())],
    });
    bar.harness.advance(Duration::from_millis(40));
    let after = bar.harness.html();
    for (name, needle, was, now) in [
        ("the first item", "id=\"tray-0\"", 0, 1),
        ("the second item", "id=\"tray-1\"", 0, 1),
        ("a status item each", "data-image=\"only\"", 4, 6),
        ("no menu open", "aria-expanded=\"true\"", 0, 0),
    ] {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}"
        );
    }
}

/// A menu of the app's bar with one row.
fn app_menu(id: i32, title: &str, row: &str) -> TrayMenuEntry {
    let leaf = |id, label: &str| TrayMenuEntry {
        id,
        kind: TrayMenuKind::Item {
            label: label.to_owned(),
            enabled: Availability::Enabled,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children: Vec::new(),
            shortcut: None,
            command: None,
        },
    };
    let mut menu = leaf(id, title);
    if let TrayMenuKind::Item { children, .. } = &mut menu.kind {
        children.push(leaf(id + 1, row));
    }
    menu
}

#[test]
fn the_focused_apps_served_menus_are_titles_after_its_name_and_an_app_without_one_has_none() {
    use sill_model::app_menu::{AppMenuCommand, AppMenus, MenuApp, MenuName, ServedMenu};
    let mut bar = start();
    let foot = MenuApp("foot".to_owned());

    // The bar tells the service whose menu it wants, and nothing is served yet.
    let told: Vec<AppMenuCommand> =
        std::iter::from_fn(|| bar.feeds.app_menu_commands.try_recv().ok()).collect();
    assert!(
        told.iter().any(
            |command| matches!(command, AppMenuCommand::Follow { app: Some(app), .. } if *app == foot)
        ),
        "{told:?}"
    );
    let before = bar.harness.html();

    bar.feeds.app_menu.send_replace(AppMenus {
        served: [(
            foot,
            ServedMenu {
                name: MenuName("org.quire.AppMenu.foot".to_owned()),
                menus: vec![
                    app_menu(11, "Foot", "About Foot"),
                    app_menu(20, "Edit", "Undo"),
                    app_menu(40, "View", "Sidebar"),
                ],
            },
        )]
        .into(),
    });
    bar.harness.advance(Duration::from_millis(40));
    let after = bar.harness.html();
    for (name, needle, was, now) in [
        ("the Edit title", ">Edit<", 0, 1),
        ("the View title", ">View<", 0, 1),
        // The app's first menu is the app name's own, not a title.
        ("no title for the first menu", ">Foot<", 0, 0),
        ("the app name stays", ">foot<", 1, 1),
        ("no menu open", "aria-expanded=\"true\"", 0, 0),
    ] {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}: {needle}\n{after}"
        );
    }

    // The service lets the app go (it quit): the titles go with it.
    bar.feeds.app_menu.send_replace(AppMenus::default());
    bar.harness.advance(Duration::from_millis(40));
    assert_eq!(count(&bar.harness.html(), ">Edit<"), 0);
}
