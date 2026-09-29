//! The popup menu in a `ds_native::Harness` larger than the popup: the frame is exactly the card
//! and its room (what `PopupSize::FitContent` measures), and a press on an item reaches the menu
//! and picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Alpha, Appearance, Availability, MaterialStack, MenuEntry, MenuTrail, ShellMetrics};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};

use super::{MenuModel, MenuPopup, PopupLook, PopupRoom};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;
const ROOM: PopupRoom = PopupRoom {
    side: 20,
    top: 8,
    bottom: 32,
};

thread_local! {
    static PICKS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Two items, each picking its own number.
#[derive(Debug, Clone, PartialEq)]
struct Numbers;

impl MenuModel for Numbers {
    type Pick = u8;

    fn entries(&self) -> Vec<MenuEntry<u8>> {
        [(1, "First"), (2, "Second")]
            .into_iter()
            .map(|(value, title)| MenuEntry::Item {
                value,
                title: title.to_owned(),
                detail: None,
                tile: None,
                trail: MenuTrail::None,
                check: None,
                availability: Availability::Enabled,
            })
            .collect()
    }
}

fn app() -> Element {
    use_context_provider(|| PopupLook {
        appearance: Appearance::default(),
        system: ds::SystemPrefs::default(),
        tint: Alpha(800),
        metrics: ShellMetrics::default(),
        stack: MaterialStack::default(),
    });
    use_context_provider(|| ROOM);
    rsx! {
        MenuPopup {
            model: Numbers,
            on_pick: move |pick: u8| PICKS.with(|p| p.borrow_mut().push(pick)),
            on_close: move |()| {},
        }
    }
}

#[test]
fn the_frame_is_the_card_and_its_room_and_a_press_picks_an_item() {
    let viewport = Viewport {
        width: WIDTH,
        height: HEIGHT,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    let frame = harness
        .rect(".sill-popup-frame")
        .expect("the frame is laid out"); // test-only
    let card = harness.rect(".ds-popover").expect("the card is laid out"); // test-only
    let px = |v: i32| f32::from(i16::try_from(v).unwrap_or(0));
    assert_eq!(
        (card.origin.x.0, card.origin.y.0),
        (px(ROOM.side), px(ROOM.top)),
        "the card sits at the room's corner"
    );
    assert_eq!(
        (frame.size.width.0, frame.size.height.0),
        (
            card.size.width.0 + 2.0 * px(ROOM.side),
            card.size.height.0 + px(ROOM.top) + px(ROOM.bottom)
        ),
        "the frame is the card plus its room, not the viewport"
    );
    assert_eq!(harness.count(".ds-menu-item"), 2);
    let second = harness
        .centre(".ds-menu-item:nth-child(2)")
        .expect("the second item is laid out"); // test-only
    harness.click(second);
    harness.advance(Duration::from_millis(100));
    assert_eq!(PICKS.with(|p| p.borrow().clone()), vec![2]);
}
