//! The dock's context menu as a [`MenuModel`]: the entries `menu_entries` built for the tile,
//! each carrying the [`DockPick`] it yields.

use ds::prelude::*;

use crate::dock::pick::DockPick;
use sill_ui_kit::menu::MenuModel;

/// One tile's context menu.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DockMenu {
    pub entries: Vec<MenuItem<DockPick>>,
}

impl MenuModel for DockMenu {
    type Pick = DockPick;

    fn entries(&self) -> Vec<MenuItem<DockPick>> {
        self.entries.clone()
    }
}
