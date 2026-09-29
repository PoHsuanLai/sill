//! The dock's context menu as a [`MenuModel`]: the entries `menu_entries` built for the tile,
//! each carrying the [`DockPick`] it yields.

use ds::MenuEntry;

use crate::menu::MenuModel;
use crate::surfaces::dock::pick::DockPick;

/// One tile's context menu.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DockMenu {
    pub entries: Vec<MenuEntry<DockPick>>,
}

impl MenuModel for DockMenu {
    type Pick = DockPick;

    fn entries(&self) -> Vec<MenuEntry<DockPick>> {
        self.entries.clone()
    }
}
