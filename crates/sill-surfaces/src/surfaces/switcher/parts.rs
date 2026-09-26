//! The adapter onto quire's M11 `AppSwitcher` (announced on quire's `m11-parts`, not landed at
//! the freeze: an Osd panel of 112 px cells with 96 px icons, the selection ring, the app name
//! as a `Tooltip{Fly}` under the selection, icons shrinking to fit an overflowing row, and a
//! quit tile folding away). Only this file changes when it lands; until then it draws nothing
//! (coherence rule 3).

#![allow(dead_code)] // Freeze: lane c mounts it from `panel.rs`.

use dioxus::prelude::*;
use sill_launcher::AppId;

/// One tile as the panel needs it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Tile {
    pub app: AppId,
    /// The app's display name, for the label under the selection.
    pub name: String,
}

/// quire's `AppSwitcher`: `selected` indexes `tiles`; `on_hover` and `on_click` name a tile.
#[component]
pub(super) fn Panel(
    tiles: Vec<Tile>,
    selected: usize,
    on_hover: EventHandler<usize>,
    on_click: EventHandler<usize>,
) -> Element {
    let _ = (tiles, selected, on_hover, on_click);
    rsx! {}
}
