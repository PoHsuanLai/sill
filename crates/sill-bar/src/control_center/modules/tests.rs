//! The module table: every kind is listed once, each answers for its span and its bar item, and
//! the bar's items stand in the order the settings offer them.

use sill_settings::{ControlCenterModule, ControlCenterSettings, InMenuBar};

use super::{KINDS, Span, module};
use crate::bar::BarMenu;
use crate::control_center::MENU_BAR_ORDER;

#[test]
fn every_kind_is_listed_once_and_the_default_grid_shows_them_all() {
    let mut listed = KINDS.to_vec();
    listed.dedup();
    assert_eq!(listed, KINDS.to_vec(), "no kind twice in a row");
    let mut shown = ControlCenterSettings::default().modules;
    shown.sort_by_key(|kind| format!("{kind:?}"));
    let mut all = KINDS.to_vec();
    all.sort_by_key(|kind| format!("{kind:?}"));
    assert_eq!(shown, all, "the default modules are every kind");
}

#[test]
fn tiles_take_a_column_and_sliders_and_panels_the_row() {
    use ControlCenterModule::{
        Appearance, Battery, Bluetooth, Display, Focus, KeyboardBrightness, NowPlaying, Sound,
        Terminal, Wifi,
    };
    const CASES: &[(ControlCenterModule, Span)] = &[
        (Wifi, Span::One),
        (Bluetooth, Span::One),
        (Focus, Span::One),
        (Display, Span::Row),
        (KeyboardBrightness, Span::Row),
        (Sound, Span::Row),
        (NowPlaying, Span::Row),
        (Appearance, Span::Row),
        (Battery, Span::Row),
        (Terminal, Span::Row),
    ];
    for &(kind, span) in CASES {
        assert_eq!(module(kind).span(), span, "{kind:?}");
    }
}

#[test]
fn a_module_is_a_bar_item_exactly_when_the_settings_can_show_it() {
    let shown = ControlCenterSettings {
        menu_bar_wifi: InMenuBar::Show,
        menu_bar_bluetooth: InMenuBar::Show,
        menu_bar_sound: InMenuBar::Show,
        menu_bar_display: InMenuBar::Show,
        menu_bar_battery: InMenuBar::Show,
        menu_bar_now_playing: InMenuBar::Show,
        ..ControlCenterSettings::default()
    };
    for kind in KINDS {
        let item = module(kind).bar_item();
        let offered = module(kind).menu_bar(&shown);
        let want = if item.is_some() {
            InMenuBar::Show
        } else {
            InMenuBar::Hide
        };
        assert_eq!(offered, want, "{kind:?}");
        assert_eq!(item.is_some(), MENU_BAR_ORDER.contains(&kind), "{kind:?}");
    }
}

#[test]
fn each_bar_item_opens_its_own_menu_and_the_menu_names_the_module_back() {
    for kind in MENU_BAR_ORDER {
        let face = module(kind).bar_item();
        assert_eq!(
            face.and_then(|face| face.menu.module()),
            Some(kind),
            "{kind:?}"
        );
    }
    let menus: Vec<BarMenu> = MENU_BAR_ORDER
        .into_iter()
        .filter_map(|kind| module(kind).bar_item().map(|face| face.menu))
        .collect();
    assert_eq!(menus.len(), MENU_BAR_ORDER.len());
}
