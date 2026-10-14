//! The desktop menu drawn by the shared popup in a `ds_harness::Harness`: its entries are
//! the model's, a press on one picks it, and the View Options submenu opens beside the menu with
//! the choice in force checked.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::material::stack::MaterialStack;
use ds::style::tokens::shell_type::ShellMetrics;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_launcher::AppId;
use sill_settings::Px;
use sill_ui_kit::links::DesktopView;
use sill_ui_kit::menu::{MenuPopup, PopupLook, PopupRoom};

use super::model::{DesktopPick, MenuWorld};

thread_local! {
    static PICKS: RefCell<Vec<DesktopPick>> = const { RefCell::new(Vec::new()) };
}

fn app() -> Element {
    use_context_provider(|| PopupLook {
        appearance: Appearance::default(),
        system: SystemPrefs::default(),
        tint: Alpha(800),
        metrics: ShellMetrics::default(),
        stack: MaterialStack::default(),
    });
    use_context_provider(|| PopupRoom::CONTEXT);
    let world = MenuWorld {
        picker: Some(AppId("org.quire.Settings".to_owned())),
        view: DesktopView {
            cell: Px(164),
            gap: Px(24),
        },
    };
    rsx! {
        MenuPopup {
            model: world,
            on_pick: move |pick: DesktopPick| PICKS.with(|p| p.borrow_mut().push(pick)),
            on_close: move |()| {},
            surface: "desktop-menu",
        }
    }
}

fn start() -> Harness {
    PICKS.with(|p| p.borrow_mut().clear());
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

#[test]
fn the_entries_are_drawn_and_a_press_picks_one() {
    let mut harness = start();
    // The rule after the second entry is the third child.
    let titles: Vec<String> = [1, 2, 4]
        .into_iter()
        .filter_map(|n| harness.text_of(&format!(".ds-menu-item:nth-child({n}) .ds-menu-label")))
        .collect();
    assert_eq!(
        titles,
        [
            "Change Wallpaper\u{2026}",
            "Edit Widgets\u{2026}",
            "Show View Options"
        ]
    );
    click(&mut harness, ".ds-menu-item:nth-child(2)");
    assert_eq!(
        PICKS.with(|p| p.borrow().clone()),
        vec![DesktopPick::EditWidgets]
    );
}

#[test]
fn view_options_open_beside_the_menu_with_the_choice_in_force_checked() {
    let mut harness = start();
    assert_eq!(harness.count(SUBMENU), 0, "closed until asked");
    click(&mut harness, PARENT);
    settle_until(&mut harness, |h| h.count(SUBMENU) == 1);
    assert_eq!(
        harness.count(SUBMENU),
        1,
        "the submenu opens on a press of its entry"
    );
    assert_eq!(
        harness.count(&format!("{SUBMENU} .ds-menu-item")),
        6,
        "three sizes and three spacings"
    );
    assert_eq!(
        harness.count(&format!("{SUBMENU} .ds-menu-item[*|aria-checked=true]")),
        2,
        "the 164 px cell and the 24 px gap are in force"
    );
    // Rows are numbered with the headers and the rule among them.
    let checked: Vec<String> = (1..=9)
        .filter(|n| {
            harness.count(&format!(
                "{SUBMENU} .ds-menu-item:nth-child({n})[*|aria-checked=true]"
            )) == 1
        })
        .filter_map(|n| {
            harness.text_of(&format!(
                "{SUBMENU} .ds-menu-item:nth-child({n}) .ds-menu-label"
            ))
        })
        .collect();
    assert_eq!(checked, ["Standard", "Wide"]);
}

const SUBMENU: &str = ".ds-menu[*|data-depth]";
const PARENT: &str = ".ds-menu-item[*|aria-haspopup=true]";
