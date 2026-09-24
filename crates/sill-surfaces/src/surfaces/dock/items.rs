//! Which tiles the dock shows, in order (design/10 §10.3.9): `[pinned (dock.json order)]
//! [running, unpinned, in launch order] | [Trash]`, running windows grouped by `match_app`.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::AppId;
use sill_services::{AppIndex, DockPins, Toplevels};
use sill_settings::TrashTile;

use crate::surfaces::dock::machine::{TileApp, TileId};

/// One tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItem {
    pub tile: TileId,
    pub kind: ItemKind,
    pub app: TileApp,
}

/// What a tile stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Pinned(AppId),
    /// A running app with no pin; `AppId` when `match_app` found an entry, else the raw app id.
    Running(AppId),
    /// A running window whose app id matches no entry.
    Unmatched(String),
    Trash,
}

/// The tiles for `pins`, `toplevels` and `index`, with the Trash tile when `trash` is on.
pub fn dock_items(
    pins: &DockPins,
    toplevels: &Toplevels,
    index: &AppIndex,
    trash: TrashTile,
) -> Vec<DockItem> {
    todo!("M3 dock-ui: pins, then running grouped by match_app in first-seen order, then Trash")
}
