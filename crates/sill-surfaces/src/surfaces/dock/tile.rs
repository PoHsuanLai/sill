//! One dock tile: the plated icon at its magnified size and bounce lift, the running dot, the
//! count badge (`Count`, `999+`), the progress ring, the pressed scrim.

#![allow(unused_variables, dead_code)] // Freeze stubs

use dioxus::prelude::*;
use sill_services::{Badge, Cpx};

use crate::surfaces::dock::items::DockItem;

/// A tile at `left` with side `size`, lifted `lift` by a bounce.
#[component]
pub fn Tile(item: DockItem, left: Cpx, size: Cpx, lift: Cpx, badge: Option<Badge>) -> Element {
    todo!("M3 dock-ui")
}
