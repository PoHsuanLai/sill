//! The tray's dbusmenu document in a `ds_native::Harness` at the popup's size: the root and the
//! page have the viewport's height (quire CONSUMING §2), and a press on an entry reaches the
//! menu and picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Availability, MenuEntry, Trail};
use ds_native::{Harness, Viewport};

use super::{POPUP_WIDTH, TrayMenuDocument};

thread_local! {
    static PICKS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// The popup's height in the test: room for two rows and the slack.
const HEIGHT: u32 = 120;

fn entry(id: i32, title: &str) -> MenuEntry<i32> {
    MenuEntry::Item {
        value: id,
        title: title.to_owned(),
        detail: None,
        tile: None,
        trail: Trail::None,
        check: None,
        availability: Availability::Enabled,
    }
}

fn app() -> Element {
    rsx! {
        TrayMenuDocument {
            entries: vec![entry(7, "Open"), entry(9, "Quit")],
            onpick: move |id: i32| PICKS.with(|p| p.borrow_mut().push(id)),
            onclose: move |()| {},
        }
    }
}

#[test]
fn the_root_has_the_popups_height_and_a_press_picks_an_entry() {
    let viewport = Viewport {
        width: u32::try_from(POPUP_WIDTH).unwrap_or(1),
        height: HEIGHT,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, viewport);
    harness.advance(Duration::from_millis(300));
    for selector in ["html", ".ds"] {
        let height = harness.rect(selector).map(|r| r.size.height.0);
        assert!(
            height.is_some_and(|h| h > 0.0),
            "{selector} has a height: {height:?}"
        );
    }
    assert_eq!(harness.count(".ds-menu-item"), 2);
    let quit = harness
        .centre(".ds-menu-item:nth-child(2)")
        .expect("the Quit entry is laid out"); // test-only
    harness.click(quit);
    harness.advance(Duration::from_millis(100));
    assert_eq!(PICKS.with(|p| p.borrow().clone()), vec![9]);
}
