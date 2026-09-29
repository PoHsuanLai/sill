//! [`menu_size`] against quire's `Menu{Dropdown}` as a Harness lays it out (`Clock::Virtual`,
//! the default appearance pinned on the root): each estimate covers the menu card with only a
//! few pixels to spare (design/29 R9).

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds::{
    Anchor, Appearance, Availability, Ds, Material, Menu, MenuEntrance, MenuEntry, MenuKind,
    MenuTrail, Point, Px,
};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};

use super::menu_size;

/// The most an estimate may overshoot the card.
const SPARE: f32 = 8.0;

thread_local! {
    static ENTRIES: RefCell<Vec<MenuEntry<u8>>> = const { RefCell::new(Vec::new()) };
}

fn item(title: &str) -> MenuEntry<u8> {
    MenuEntry::Item {
        value: 0,
        title: title.to_owned(),
        detail: None,
        tile: None,
        trail: MenuTrail::None,
        check: None,
        availability: Availability::Enabled,
    }
}

fn info(title: &str, detail: Option<&str>) -> MenuEntry<u8> {
    MenuEntry::Info {
        title: title.to_owned(),
        detail: detail.map(str::to_owned),
    }
}

fn app() -> Element {
    let entries = ENTRIES.with(|e| e.borrow().clone());
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover,
            Menu {
                kind: MenuKind::Dropdown,
                anchor: Anchor::Point(Point { x: Px(8.0), y: Px(1.0) }),
                entries,
                onpick: move |_: u8| {},
                onclose: move |()| {},
                entrance: MenuEntrance::Instant,
            }
        }
    }
}

/// The card's laid-out height for `entries`.
fn laid(entries: Vec<MenuEntry<u8>>) -> f32 {
    ENTRIES.with(|e| *e.borrow_mut() = entries);
    let viewport = Viewport {
        width: 480,
        height: 800,
        scale_percent: 100,
    };
    let mut harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(300));
    harness
        .rect(".ds-menu")
        .map(|rect| rect.size.height.0)
        .unwrap_or_else(|| panic!("the menu is laid out: {}", harness.html()))
}

#[test]
fn each_menu_estimate_covers_its_card() {
    let cases: Vec<(&str, Vec<MenuEntry<u8>>)> = vec![
        ("one item", vec![item("About This Computer")]),
        ("two items", vec![item("Mirror"), item("Extend")]),
        (
            "a header alone",
            vec![MenuEntry::Header("Clock".to_owned())],
        ),
        (
            "a two-line info",
            vec![info("Battery", Some("2:05 remaining"))],
        ),
        (
            "an app menu",
            vec![
                item("About Files"),
                MenuEntry::Separator,
                item("Settings…"),
                MenuEntry::Separator,
                item("Hide Files"),
                item("Hide Others"),
                item("Show All"),
                MenuEntry::Separator,
                item("Quit Files"),
            ],
        ),
        (
            "a header over items",
            vec![
                MenuEntry::Header("Clock".to_owned()),
                item("Open Date & Time Settings…"),
            ],
        ),
        (
            "info lines",
            vec![
                info("Battery", Some("2:05 remaining")),
                info("Power Source: Battery", None),
                MenuEntry::Separator,
                item("Battery Settings…"),
            ],
        ),
    ];
    let misses: Vec<String> = cases
        .into_iter()
        .filter_map(|(name, entries)| {
            let estimate = menu_size(&entries).height as f32;
            let laid = laid(entries);
            let spare = estimate - laid;
            match (0.0..SPARE).contains(&spare) {
                true => None,
                false => Some(format!(
                    "{name}: estimate {estimate} against the laid-out {laid}"
                )),
            }
        })
        .collect();
    assert!(misses.is_empty(), "{misses:#?}");
}
