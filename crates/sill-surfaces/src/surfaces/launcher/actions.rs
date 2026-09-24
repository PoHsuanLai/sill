//! The actions menu (Ctrl+K / Alt+K): a quire `Menu{Rich}` over the selected row's actions.

#![allow(unused_variables, dead_code)] // Freeze stubs

use dioxus::prelude::*;
use sill_launcher::Action;

/// `actions`, `selected` highlighted.
#[component]
pub fn ActionsList(actions: Vec<Action>, selected: usize) -> Element {
    todo!("M4 launcher-ui")
}
