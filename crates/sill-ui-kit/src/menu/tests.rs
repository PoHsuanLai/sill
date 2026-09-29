//! The popup menu in a `ds_native::Harness` at the popup's own size: the frame, the root and
//! the page have the viewport's height, and a press on an item reaches the menu and picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{
    Alpha, Anchor, Appearance, Availability, MaterialStack, MenuEntry, MenuTrail, Point, Px,
    ShellMetrics,
};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};

use super::{MenuModel, MenuPopup, PopupLook, PopupRoom};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 260;

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
    use_context_provider(|| PopupRoom {
        side: 20,
        top: 0,
        bottom: 32,
    });
    rsx! {
        MenuPopup {
            model: Numbers,
            anchor: Anchor::Point(Point {
                x: Px(28.0),
                y: Px(200.0),
            }),
            on_pick: move |pick: u8| PICKS.with(|p| p.borrow_mut().push(pick)),
            on_close: move |()| {},
        }
    }
}

#[test]
fn the_frame_has_the_popups_height_and_a_press_picks_an_item() {
    let viewport = Viewport {
        width: WIDTH,
        height: HEIGHT,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    for selector in ["html", ".sill-menu-popup-frame", ".ds"] {
        let rect = harness.rect(selector).map(|r| r.size.height.0);
        assert!(
            rect.is_some_and(|h| h > 0.0),
            "{selector} has a height: {rect:?}"
        );
    }
    let frame = harness
        .rect(".sill-menu-popup-frame")
        .map(|r| r.size.height.0);
    assert_eq!(
        frame,
        Some(f32::from(u16::try_from(HEIGHT).unwrap_or(0))),
        "the frame is the popup"
    );
    assert_eq!(harness.count(".ds-menu-item"), 2);
    let second = harness
        .centre(".ds-menu-item:nth-child(2)")
        .expect("the second item is laid out"); // test-only
    harness.click(second);
    harness.advance(Duration::from_millis(100));
    assert_eq!(PICKS.with(|p| p.borrow().clone()), vec![2]);
}
