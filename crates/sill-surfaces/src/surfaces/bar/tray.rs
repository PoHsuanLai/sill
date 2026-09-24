//! The tray: one `IconButton{Tool}` per status notifier item, its dbusmenu as a quire `Menu` (M1 svc-tray owns this file).

#![allow(unused_variables, dead_code)] // Freeze stubs

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The tray: one `IconButton{Tool}` per status notifier item, its dbusmenu as a quire `Menu` (M1 svc-tray owns this file).
#[component]
pub fn Tray(output: OutputInfo) -> Element {
    todo!("M1 svc-tray")
}
