//! The window menu drawn by the shared popup in a `ds_harness::Harness`: the top level is
//! Exit or Enter Full Screen, the two submenus, Fill and Center; the entries the compositor's
//! bit set does not offer are drawn dimmed, not left out; a press on an offered one picks it,
//! and the submenu opens beside the menu with its entries in order.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::material::stack::MaterialStack;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_model::desktop::{WindowEntries, WindowEntry};
use sill_ui_kit::menu::{MenuPopup, PopupLook, PopupRoom};

use super::model::WindowMenuModel;

thread_local! {
    static PICKS: RefCell<Vec<WindowEntry>> = const { RefCell::new(Vec::new()) };
    static ENABLED: RefCell<WindowEntries> = RefCell::new(WindowEntries::default());
}

fn app() -> Element {
    use_context_provider(|| PopupLook {
        appearance: Appearance::default(),
        system: SystemPrefs::default(),
        tint: Alpha(800),
        stack: MaterialStack::default(),
    });
    use_context_provider(|| PopupRoom::CONTEXT);
    rsx! {
        MenuPopup {
            model: WindowMenuModel { enabled: ENABLED.with(|bits| bits.borrow().clone()) },
            on_pick: move |entry: WindowEntry| PICKS.with(|picks| picks.borrow_mut().push(entry)),
            on_close: move |()| {},
            surface: "window-menu",
        }
    }
}

/// The popup over a window that offers `offered`.
fn start(offered: &[WindowEntry]) -> Harness {
    PICKS.with(|picks| picks.borrow_mut().clear());
    ENABLED.with(|bits| {
        *bits.borrow_mut() = offered.iter().copied().collect();
    });
    let viewport = Viewport {
        width: 640,
        height: 480,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    harness
}

fn click(harness: &mut Harness, selector: &str) {
    let at = harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out"));
    harness.send(Input::click(at));
    harness.advance(Duration::from_millis(300));
}

const ITEM: &str = ".ds-menu-item";
const SUBMENU: &str = ".ds-menu[*|data-depth]";
const PARENT: &str = ".ds-menu-item[*|aria-haspopup=true]";

#[test]
fn the_top_level_keeps_every_entry_and_dims_the_ones_the_window_cannot_take() {
    use WindowEntry::{Centre, EnterFullScreen, Fill, Left};
    let harness = start(&[EnterFullScreen, Left, Fill, Centre]);
    let titles: Vec<String> = (1..=5)
        .filter_map(|n| harness.text_of(&format!("{ITEM}:nth-child({n}) .ds-menu-label")))
        .collect();
    assert_eq!(
        titles,
        [
            "Enter Full Screen",
            "Move & Resize",
            "Full Screen Tile",
            "Fill",
            "Center"
        ]
    );
    // Full Screen Tile is the only one with nothing to offer.
    assert_eq!(harness.count(&format!("{ITEM}[*|aria-disabled=true]")), 1);
    assert_eq!(
        harness.count(&format!("{ITEM}:nth-child(3)[*|aria-disabled=true]")),
        1,
        "the Full Screen Tile submenu is the dimmed one"
    );
}

#[test]
fn a_window_in_full_screen_offers_exit_and_dims_the_rest() {
    let harness = start(&[WindowEntry::ExitFullScreen]);
    assert_eq!(
        harness.text_of(&format!("{ITEM}:nth-child(1) .ds-menu-label")),
        Some("Exit Full Screen".to_owned())
    );
    assert_eq!(
        harness.count(&format!("{ITEM}[*|aria-disabled=true]")),
        4,
        "everything but Exit Full Screen"
    );
}

#[test]
fn a_press_on_an_offered_entry_picks_it_and_a_dimmed_one_picks_nothing() {
    use WindowEntry::{EnterFullScreen, Fill};
    let mut harness = start(&[EnterFullScreen, Fill]);
    click(&mut harness, &format!("{ITEM}:nth-child(4)"));
    assert_eq!(PICKS.with(|picks| picks.borrow().clone()), vec![Fill]);
    let mut harness = start(&[EnterFullScreen]);
    click(&mut harness, &format!("{ITEM}:nth-child(4)"));
    assert!(
        PICKS.with(|picks| picks.borrow().is_empty()),
        "Fill is dimmed"
    );
}

#[test]
fn move_and_resize_opens_beside_the_menu_and_a_press_in_it_picks_left() {
    let mut harness = start(&[WindowEntry::Left, WindowEntry::Right]);
    assert_eq!(harness.count(SUBMENU), 0, "closed until asked");
    click(&mut harness, PARENT);
    settle_until(&mut harness, |h| h.count(SUBMENU) == 1);
    assert_eq!(harness.count(SUBMENU), 1);
    let titles: Vec<String> = [1, 2]
        .into_iter()
        .filter_map(|n| harness.text_of(&format!("{SUBMENU} {ITEM}:nth-child({n}) .ds-menu-label")))
        .collect();
    assert_eq!(titles, ["Left", "Right"]);
    click(&mut harness, &format!("{SUBMENU} {ITEM}:nth-child(1)"));
    assert_eq!(
        PICKS.with(|picks| picks.borrow().clone()),
        vec![WindowEntry::Left]
    );
}
