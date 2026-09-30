//! The tray's dbusmenu document in a `ds_native::Harness` larger than the popup: the frame is
//! the card and its inset (what `PopupSize::FitContent` measures), and a press on an entry
//! reaches the menu and picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Availability, MenuItem};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};

use super::{INSET, TrayMenuDocument};

thread_local! {
    static PICKS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// The harness viewport: bigger than the popup.
const SIZE: u32 = 300;

fn entry(id: i32, title: &str) -> MenuItem<i32> {
    MenuItem::Item {
        value: id,
        title: title.to_owned(),
        image: None,
        key: None,
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
fn the_frame_is_the_card_and_its_inset_and_a_press_picks_an_entry() {
    let viewport = Viewport {
        width: SIZE,
        height: SIZE,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    let frame = harness
        .rect(".sill-popup-frame")
        .expect("the frame is laid out"); // test-only
    let card = harness.rect(".ds-popover").expect("the card is laid out"); // test-only
    let inset = f32::from(i16::try_from(INSET.side).unwrap_or(0));
    assert_eq!(
        (frame.size.width.0, frame.size.height.0),
        (
            card.size.width.0 + 2.0 * inset,
            card.size.height.0 + 2.0 * inset
        ),
        "the frame is the card plus its inset, not the viewport"
    );
    assert_eq!(harness.count(".ds-menu-item"), 2);
    let quit = harness
        .centre(".ds-menu-item:nth-child(2)")
        .expect("the Quit entry is laid out"); // test-only
    harness.click(quit);
    harness.advance(Duration::from_millis(300));
    assert_eq!(PICKS.with(|p| p.borrow().clone()), vec![9]);
}

#[test]
fn a_key_turns_the_tray_menus_ring_on_and_a_hover_keeps_it() {
    let viewport = Viewport {
        width: SIZE,
        height: SIZE,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    let modality = |harness: &Harness| harness.attr(".ds", "data-modality");
    assert_eq!(modality(&harness).as_deref(), Some("pointer"), "at first");
    harness.key(ds::ShortcutKey::Down);
    harness.advance(Duration::from_millis(300));
    assert_eq!(
        modality(&harness).as_deref(),
        Some("keyboard"),
        "after a key"
    );
    let open = harness
        .centre(".ds-menu-item:nth-child(1)")
        .expect("the Open entry is laid out"); // test-only
    harness.pointer_move(open);
    harness.advance(Duration::from_millis(300));
    assert_eq!(
        modality(&harness).as_deref(),
        Some("keyboard"),
        "a move is not a press"
    );
}
