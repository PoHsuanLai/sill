//! The popup menu in a `ds_harness::Harness` larger than the popup: the frame is exactly the card
//! and its room (what `PopupSize::FitContent` measures), and a press on an item reaches the menu
//! and picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use ds_settings::UserStyle;
use ds_style::material::stack::MaterialStack;
use ds_style::tokens::shell_type::ShellMetrics;
use sill_settings::ShellFile;

use crate::settings::SettingsFeed;

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

    fn entries(&self) -> Vec<MenuItem<u8>> {
        [(1, "First"), (2, "Second")]
            .into_iter()
            .map(|(value, title)| MenuItem::Item {
                value,
                title: title.to_owned(),
                image: None,
                key: None,
                check: None,
                availability: Availability::Enabled,
            })
            .collect()
    }
}

fn app() -> Element {
    provide_popup_contexts();
    popup()
}

/// The popup as its opener leaves it, plus the person's stylesheet on the feed it hands over.
fn styled_app() -> Element {
    use_context_provider(|| SettingsFeed {
        user_style: tokio::sync::watch::channel(UserStyle(STYLE.to_owned())).1,
        ..SettingsFeed::fixed(ShellFile::default())
    });
    provide_popup_contexts();
    popup()
}

const STYLE: &str = ".ds-popover { color: red }";

fn provide_popup_contexts() {
    use_context_provider(|| PopupLook {
        appearance: Appearance::default(),
        system: SystemPrefs::default(),
        tint: Alpha(800),
        metrics: ShellMetrics::default(),
        stack: MaterialStack::default(),
    });
    use_context_provider(|| ROOM);
}

fn popup() -> Element {
    rsx! {
        MenuPopup {
            model: Numbers,
            on_pick: move |pick: u8| PICKS.with(|p| p.borrow_mut().push(pick)),
            on_close: move |()| {},
            surface: "test-menu",
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
    let mut harness = Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
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
    harness.send(Input::click(second));
    harness.advance(Duration::from_millis(300));
    assert_eq!(PICKS.with(|p| p.borrow().clone()), vec![2]);
}

#[test]
fn a_popup_names_its_surface_and_draws_the_person_style_its_opener_handed_over() {
    let viewport = Viewport {
        width: WIDTH,
        height: HEIGHT,
        scale_percent: 100,
    };
    let start = |app: fn() -> Element| {
        let mut harness =
            Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
        harness.advance(Duration::from_millis(100));
        harness
    };
    let bare = start(app);
    assert_eq!(
        bare.attr(".ds", "data-surface").as_deref(),
        Some("test-menu")
    );
    assert_eq!(
        bare.html().matches("data-ds-user").count(),
        0,
        "no feed: no style"
    );
    let styled = start(styled_app);
    assert_eq!(
        styled.attr(".ds", "data-surface").as_deref(),
        Some("test-menu")
    );
    assert_eq!(styled.html().matches("data-ds-user").count(), 1);
    assert!(styled.html().contains(STYLE), "{}", styled.html());
}
