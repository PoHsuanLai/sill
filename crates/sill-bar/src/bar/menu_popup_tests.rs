//! The bar popup's document on a tray item's menu: the submenu opens beside its row after the
//! rest delay inside the popup's own frame (which `PopupSize::FitContent` measures), and a click
//! on a submenu row picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds::stack::menu_track::types::MenuTiming;
use ds::style::material::stack::MaterialStack;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use shell_host::{PopupPlacement, Rect as HostRect, Size as HostSize};
use sill_model::tray::{Check, TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind};
use sill_model::units::{Availability as Enabled, Visibility};
use sill_ui_kit::menu::PopupLook;
use tokio::sync::{mpsc, watch};

use super::{PopupDocument, PopupLink, SUBMENU_ROOM, submenu_room};
use crate::bar::content::PopupBody;
use crate::bar::dbusmenu::menu_items;
use crate::bar::menus::{BarMenu, TraySlot};
use crate::bar::model::PopupView;
use crate::bar::pick::BarPick;
use crate::bar::plan::PopupReport;

thread_local! {
    static REPORTS: RefCell<Option<mpsc::UnboundedReceiver<PopupReport>>> =
        const { RefCell::new(None) };
}

/// Wider than the popup, as the harness's viewport is not the popup's surface.
const VIEW: Viewport = Viewport {
    width: 800,
    height: 400,
    scale_percent: 100,
};

const SUBMENU: &str = ".ds-menu[*|data-depth]";
const PARENT: &str = ".ds-menu-item[*|aria-haspopup=true]";

/// What a row of the test item picks.
fn tray_pick(entry: i32) -> BarPick {
    BarPick::Tray {
        item: item_id(),
        entry,
    }
}

fn item_id() -> TrayItemId {
    TrayItemId(":1.9/StatusNotifierItem".to_owned())
}

fn row(id: i32, label: &str, children: Vec<TrayMenuEntry>) -> TrayMenuEntry {
    TrayMenuEntry {
        id,
        kind: TrayMenuKind::Item {
            label: label.to_owned(),
            enabled: Enabled::Enabled,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children,
            shortcut: None,
            command: None,
        },
    }
}

/// Open, Devices (Ping, Share), Quit: the shape of KDE Connect's menu.
fn rows() -> Vec<TrayMenuEntry> {
    vec![
        row(1, "Open", Vec::new()),
        row(
            2,
            "Devices",
            vec![row(3, "Ping", Vec::new()), row(4, "Share", Vec::new())],
        ),
        row(7, "Quit", Vec::new()),
    ]
}

fn app() -> Element {
    document(rows())
}

/// A menu whose submenu has a submenu: Open, Devices > (Phone > (Ping), Share).
fn deep_app() -> Element {
    document(vec![
        row(1, "Open", Vec::new()),
        row(
            2,
            "Devices",
            vec![
                row(3, "Phone", vec![row(4, "Ping", Vec::new())]),
                row(5, "Share", Vec::new()),
            ],
        ),
    ])
}

fn document(rows: Vec<TrayMenuEntry>) -> Element {
    let (reports, receiver) = mpsc::unbounded_channel();
    REPORTS.with(|slot| *slot.borrow_mut() = Some(receiver));
    use_context_provider(|| PopupLink {
        view: watch::channel(None).1,
        reports,
        timing: MenuTiming::default(),
        output: None,
    });
    let current = PopupView {
        menu: BarMenu::Tray(TraySlot(0)),
        body: PopupBody::Menu(menu_items(&tray_pick, &rows, &|_| None)),
        placement: PopupPlacement {
            anchor_rect: HostRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            size: HostSize {
                width: 1,
                height: 1,
            },
        },
        look: PopupLook {
            appearance: Appearance::default(),
            system: SystemPrefs::default(),
            tint: Alpha(800),
            stack: MaterialStack::default(),
        },
    };
    rsx! {
        PopupDocument { current }
    }
}

fn harness() -> Harness {
    started(app)
}

fn started(app: fn() -> Element) -> Harness {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(100));
    harness
}

fn reports() -> Vec<PopupReport> {
    REPORTS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let receiver = slot.as_mut().expect("the popup was mounted"); // test-only
        std::iter::from_fn(|| receiver.try_recv().ok()).collect()
    })
}

#[test]
fn a_tray_submenu_opens_beside_its_row_after_the_rest_delay_inside_the_popup() {
    let mut harness = harness();
    assert_eq!(harness.count(SUBMENU), 0, "closed to begin with");
    let parent = harness.centre(PARENT).expect("the Devices row is laid out"); // test-only
    harness.send(Input::pointer_move(parent));
    harness.advance(Duration::from_millis(100));
    assert_eq!(harness.count(SUBMENU), 0, "not before the delay");
    harness.advance(Duration::from_millis(200));
    assert_eq!(harness.count(SUBMENU), 1, "after it:\n{}", harness.html());
    harness.advance(Duration::from_millis(100));
    let frame = harness.rect(".sill-popup-frame").expect("the frame"); // test-only
    let card = harness
        .rect(".ds-popover:not([*|data-depth])")
        .expect("the card"); // test-only
    let sub = harness.rect(SUBMENU).expect("the submenu"); // test-only
    let beside = sub.origin.x.0 >= card.origin.x.0 + card.size.width.0
        || sub.origin.x.0 + sub.size.width.0 <= card.origin.x.0;
    assert!(beside, "beside the menu, not over it: {sub:?} for {card:?}");
    assert!(
        sub.origin.x.0 >= frame.origin.x.0
            && sub.origin.x.0 + sub.size.width.0 <= frame.origin.x.0 + frame.size.width.0,
        "inside the popup's frame, which is what the surface is fitted to: {sub:?} in {frame:?}"
    );
    let row = harness.rect(PARENT).expect("the Devices row"); // test-only
    // The submenu is as tall as three of the card's rows, so quire's 8 px margin lifts it a little
    // above its row; it still stands against the row, inside the card's height.
    let (top, bottom) = (sub.origin.y.0, sub.origin.y.0 + sub.size.height.0);
    assert!(
        top <= row.origin.y.0
            && bottom >= row.origin.y.0 + row.size.height.0
            && top >= card.origin.y.0
            && bottom <= card.origin.y.0 + card.size.height.0,
        "against its row, inside the menu's height, not below it: {sub:?} for {row:?} in {card:?}"
    );
}

#[test]
fn a_click_on_a_submenu_row_picks_it_by_its_dbusmenu_id() {
    let mut harness = harness();
    let parent = harness.centre(PARENT).expect("the Devices row"); // test-only
    harness.send(Input::pointer_move(parent));
    harness.advance(Duration::from_millis(400));
    let ping = harness
        .centre(&format!("{SUBMENU} .ds-menu-item:nth-child(1)"))
        .expect("the Ping row is laid out"); // test-only
    harness.send(Input::click(ping));
    harness.advance(Duration::from_millis(400));
    let picks: Vec<_> = reports()
        .into_iter()
        .filter_map(|report| match report {
            PopupReport::Picked(pick) => Some(pick),
            _ => None,
        })
        .collect();
    assert_eq!(
        picks,
        vec![BarPick::Tray {
            item: item_id(),
            entry: 3
        }]
    );
}

#[test]
fn a_menu_keeps_a_column_of_room_for_each_level_of_submenu() {
    let entries = |rows: &[TrayMenuEntry]| menu_items(&tray_pick, rows, &|_| None);
    let deep = vec![row(
        1,
        "A",
        vec![row(
            2,
            "B",
            vec![row(3, "C", vec![row(4, "leaf", Vec::new())])],
        )],
    )];
    let cases: Vec<(&str, Vec<TrayMenuEntry>, i32)> = vec![
        ("no rows", vec![], 0),
        ("plain rows", vec![row(1, "Open", Vec::new())], 0),
        ("one level", rows(), SUBMENU_ROOM),
        ("three levels", deep, 3 * SUBMENU_ROOM),
    ];
    for (name, rows, want) in cases {
        assert_eq!(submenu_room(&entries(&rows)), want, "{name}");
    }
}

#[test]
fn submenus_of_submenus_each_take_a_column_of_the_popup() {
    let mut harness = started(deep_app);
    let first = harness.centre(PARENT).expect("the Devices row"); // test-only
    harness.send(Input::pointer_move(first));
    harness.advance(Duration::from_millis(400));
    let second = harness
        .centre(&format!("{SUBMENU} {PARENT}"))
        .expect("the Phone row is laid out"); // test-only
    harness.send(Input::pointer_move(second));
    harness.advance(Duration::from_millis(400));
    assert_eq!(harness.count(SUBMENU), 2, "{}", harness.html());
    let frame = harness.rect(".sill-popup-frame").expect("the frame"); // test-only
    let card = harness
        .rect(".ds-popover:not([*|data-depth])")
        .expect("the card"); // test-only
    let first = harness
        .rect(".ds-menu[*|data-depth=\"1\"]")
        .expect("the first submenu"); // test-only
    let second = harness
        .rect(".ds-menu[*|data-depth=\"2\"]")
        .expect("the second submenu"); // test-only
    let edges = |r: &ds::prelude::Rect| (r.origin.x.0, r.origin.x.0 + r.size.width.0);
    let (frame_left, frame_right) = edges(&frame);
    let mut columns = [edges(&second), edges(&first), edges(&card)];
    columns.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(
        columns.windows(2).all(|pair| pair[0].1 <= pair[1].0),
        "left to right with none over another: {columns:?}"
    );
    assert!(
        columns[0].0 >= frame_left && columns[2].1 <= frame_right,
        "all inside the frame: {columns:?} in {frame:?}"
    );
}
