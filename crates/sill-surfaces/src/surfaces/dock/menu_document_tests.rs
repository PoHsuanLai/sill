//! The dock menu's document in a `ds_native::Harness` at the popup's own size: the frame, the
//! root and the page have the viewport's height, and a press on an item reaches the menu and
//! picks it.

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Alpha, Appearance, Availability, MaterialStack, MenuEntry, ShellMetrics, Trail};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_launcher::WindowKey;
use sill_services::Cpx;
use tokio::sync::mpsc;

use super::MenuDocument;
use crate::surfaces::dock::frame::Area;
use crate::surfaces::dock::menu_geometry::menu_placement;
use crate::surfaces::dock::menu_popup::{MenuLink, PopupLook, PopupReport};
use crate::surfaces::dock::pick::DockPick;

thread_local! {
    static LINK: RefCell<Option<MenuLink>> = const { RefCell::new(None) };
    static PICKS: RefCell<Vec<DockPick>> = const { RefCell::new(Vec::new()) };
}

fn item(window: u64, title: &str) -> MenuEntry<DockPick> {
    MenuEntry::Item {
        value: DockPick::Activate(WindowKey(window)),
        title: title.to_owned(),
        detail: None,
        tile: None,
        trail: Trail::None,
        check: None,
        availability: Availability::Enabled,
    }
}

fn app() -> Element {
    let link = LINK.with(|l| l.borrow().clone());
    let Some(link) = link else {
        return rsx! {};
    };
    use_context_provider(|| link);
    rsx! {
        MenuDocument {
            on_pick: move |pick: DockPick| PICKS.with(|p| p.borrow_mut().push(pick)),
            on_close: move |()| {},
        }
    }
}

#[test]
fn the_frame_has_the_popups_height_and_a_press_picks_an_item() {
    let entries = vec![item(1, "First"), item(2, "Second")];
    let tile = Area {
        left: Cpx::px(900),
        top: Cpx::px(96),
        width: Cpx::px(48),
        height: Cpx::px(48),
    };
    let size = menu_placement(tile, &entries).size;
    let (reports, _kept) = mpsc::unbounded_channel::<PopupReport>();
    LINK.with(|l| {
        *l.borrow_mut() = Some(MenuLink {
            entries,
            size,
            look: PopupLook {
                appearance: Appearance::default(),
                system: ds::SystemPrefs::default(),
                tint: Alpha(800),
                metrics: ShellMetrics::default(),
                stack: MaterialStack::default(),
            },
            reports,
        });
    });
    let viewport = Viewport {
        width: u32::try_from(size.width).unwrap_or(1),
        height: u32::try_from(size.height).unwrap_or(1),
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    let height = f32::from(u16::try_from(size.height).unwrap_or(0));
    for selector in ["html", ".sill-dock-popup-frame", ".ds"] {
        let rect = harness.rect(selector).map(|r| r.size.height.0);
        assert!(
            rect.is_some_and(|h| h > 0.0),
            "{selector} has a height: {rect:?}"
        );
    }
    let frame = harness
        .rect(".sill-dock-popup-frame")
        .map(|r| r.size.height.0);
    assert_eq!(frame, Some(height), "the frame is the popup");
    assert_eq!(harness.count(".ds-menu-item"), 2);
    let second = harness
        .centre(".ds-menu-item:nth-child(2)")
        .expect("the second item is laid out"); // test-only
    harness.click(second);
    harness.advance(Duration::from_millis(100));
    let picks = PICKS.with(|p| p.borrow().clone());
    assert_eq!(picks, vec![DockPick::Activate(WindowKey(2))]);
}
