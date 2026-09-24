//! The actions menu (Ctrl+K / Alt+K): a quire `Menu{Rich}` over the selected row's actions.
//!
//! The panel opens this menu with its handlers (`panel.rs`); this component is the same menu
//! with none, for a demo or a picture. The ds `Menu` owns its selection, so `selected` cannot
//! reach it (FINDINGS F141).

use dioxus::prelude::*;
use ds::{Anchor, Menu, MenuKind};
use sill_launcher::{Action, ActionId};

use crate::surfaces::launcher::entries::action_entries;
use crate::surfaces::launcher::panel::ACTIONS_AT;

/// `actions`, `selected` highlighted.
#[component]
pub fn ActionsList(actions: Vec<Action>, selected: usize) -> Element {
    let _menu_owns_the_selection = selected;
    rsx! {
        Menu::<ActionId> {
            kind: MenuKind::Rich,
            anchor: Anchor::Point(ACTIONS_AT),
            entries: action_entries(&actions),
            onpick: move |_| {},
            onclose: move |()| {},
        }
    }
}
