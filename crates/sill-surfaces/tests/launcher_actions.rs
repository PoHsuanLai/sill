//! The actions menu and the selection on the real panel in a `ds_native::Harness`: the menu
//! hangs from the selected row's rect, holds that row's actions, and on Escape gives the field
//! the keyboard back without remounting the palette; a row the pointer moves over is the
//! selection Enter runs.
//!
//! "fi" finds, best first: Firefox (the top hit), Files (Applications), "Firefox — sill
//! launcher" (Windows), Wi-Fi (Settings). Firefox has two actions, Files one.

mod launcher_harness;

use ds::Key;
use launcher_harness::launcher_support::{self, FIREFOX_WINDOW};
use launcher_harness::{is_open, open, selected_title, settle};
use sill_launcher::WindowKey;
use sill_services::{LaunchCommand, WlCommand};
use sill_surfaces::surfaces::launcher::LauncherKey;

/// The selected row (attribute selectors need the namespace wildcard on Blitz, spike S2).
const SELECTED: &str = ".ds-menu-item[*|aria-selected=true]";

#[test]
fn the_actions_menu_hangs_from_the_selected_row_and_gives_the_keyboard_back() {
    let mut open = open("fi");
    open.harness.key(Key::Down);
    settle(&mut open.harness, |h| {
        selected_title(h).as_deref() == Some("Files")
    });
    // quire's palette reports an already laid-out row's rect in the frame it was selected (G310),
    // so Ctrl+K right after the selection hangs the menu from that row.
    let row = open.harness.rect(SELECTED).expect("the selected row");
    open.scripted(LauncherKey::Actions);
    settle(&mut open.harness, |h| h.count(".ds-popover") == 1);
    let menu = open.harness.rect(".ds-popover").expect("the actions menu");
    let html = open.harness.html();
    assert!(html.contains("New Window"), "Files' action");
    assert!(
        !html.contains("New Private Window"),
        "not the top hit's actions"
    );
    let below = menu.origin.y.0 - (row.origin.y.0 + row.size.height.0);
    assert!(
        (0.0..=12.0).contains(&below),
        "the menu hangs under the row: row {row:?}, menu {menu:?}"
    );
    open.harness.key(Key::Escape);
    settle(&mut open.harness, |h| {
        h.count(".ds-popover") == 0 && h.is_focused("input")
    });
    assert_eq!(open.harness.count(".ds-popover"), 0, "the menu closed");
    assert!(is_open(&open.link), "the launcher stays");
    assert!(
        open.harness.is_focused("input"),
        "the field has the keyboard"
    );
    // The palette's entrance (cmdk-in since the polish pass) may still be running when the
    // menu closes; wait for it to settle rather than asserting the phase at once. A remount
    // would restart the query and selection, which the next assertions cover.
    settle(&mut open.harness, |h| {
        h.attr(".ds-palette", "data-presence").as_deref() == Some("present")
    });
    assert_eq!(
        open.harness.attr(".ds-palette", "data-presence").as_deref(),
        Some("present"),
        "the palette's entrance never settled"
    );
    assert_eq!(selected_title(&open.harness).as_deref(), Some("Files"));
    open.harness.key(Key::Enter);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    match open.feeds.bar.launch.try_recv() {
        Ok(LaunchCommand::Launch(request)) => {
            assert_eq!(request.app.0, launcher_support::FILES);
        }
        other => panic!("no launch: {other:?}"),
    }
}

#[test]
fn the_row_under_the_pointer_is_the_selection() {
    let mut open = open("fi");
    let rows = open.harness.count(".ds-menu-item");
    assert_eq!(rows, 4);
    let card = open.harness.rect(".ds-palette").expect("the card");
    let first = open.harness.rect(".ds-menu-item").expect("the first row");
    // Rows are equally tall under their headings; walk down until the window row is selected.
    let mut y = first.origin.y.0 + first.size.height.0 / 2.0;
    let x = first.origin.x.0 + first.size.width.0 / 2.0;
    while selected_title(&open.harness).as_deref() != Some("Firefox — sill launcher")
        && y < card.origin.y.0 + card.size.height.0
    {
        open.harness.pointer_move(ds::Point {
            x: ds::Px(x),
            y: ds::Px(y),
        });
        open.harness.advance(std::time::Duration::from_millis(20));
        y += 4.0;
    }
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("Firefox — sill launcher")
    );
    open.harness.key(Key::Enter);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    assert_eq!(
        open.feeds.bar.wl.try_recv().ok(),
        Some(WlCommand::Activate(WindowKey(FIREFOX_WINDOW)))
    );
}
