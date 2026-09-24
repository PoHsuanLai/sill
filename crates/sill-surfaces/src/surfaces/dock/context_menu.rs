//! The dock's context menu (design/10 §10.3.7): windows, desktop actions, Options (Keep in
//! Dock, Open at Login, Show in Files), Hide, Quit / Force Quit; order per
//! `dock.context_menu_order`.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_services::{App, Toplevels};
use sill_settings::DockMenuOrder;

use crate::surfaces::dock::items::DockItem;

/// One row of the menu, before it becomes quire `MenuEntry`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockMenuRow {
    Header(String),
    Window {
        title: String,
        active: Checked,
        minimized: Minimized,
    },
    DesktopAction {
        id: String,
        name: String,
    },
    KeepInDock(Checked),
    OpenAtLogin(Checked),
    ShowInFiles,
    Hide,
    Quit,
    ForceQuit,
    Separator,
}

/// A check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checked {
    Checked,
    Unchecked,
}

/// Whether a listed window is minimized (prefixed with the diamond glyph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Minimized {
    Minimized,
    Normal,
}

/// Whether Option is held while the menu is open (Quit reads Force Quit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionHeld {
    Held,
    Up,
}

/// The rows for `item`.
pub fn context_menu(
    item: &DockItem,
    app: Option<&App>,
    toplevels: &Toplevels,
    order: DockMenuOrder,
    option: OptionHeld,
) -> Vec<DockMenuRow> {
    todo!("M3 dock-ui")
}
