//! The actions menu (Ctrl+K / Alt+K): a quire `Menu{Rich}` over the selected row's actions.
//!
//! The panel opens this menu with its handlers (`panel.rs`); this component is the same menu
//! with none, for a demo or a picture. The ds `Menu` owns its selection, so `selected` cannot
//! reach it (FINDINGS F141).

use dioxus::prelude::*;
use ds::{Anchor, Menu, MenuKind, Point, Px};
use sill_launcher::{Action, ActionId};

use crate::surfaces::launcher::entries::action_entries;

/// Where the picture's menu hangs: the panel hangs it from the selected row's rect instead.
const PICTURE_AT: Point = Point {
    x: Px(330.0),
    y: Px(131.0),
};

/// `actions`, `selected` highlighted.
#[component]
pub fn ActionsList(actions: Vec<Action>, selected: usize) -> Element {
    let _menu_owns_the_selection = selected;
    rsx! {
        Menu::<ActionId> {
            kind: MenuKind::Rich,
            anchor: Anchor::Point(PICTURE_AT),
            entries: action_entries(&actions),
            onpick: move |_| {},
            onclose: move |()| {},
        }
    }
}
