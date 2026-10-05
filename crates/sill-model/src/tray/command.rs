//! What the tray service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayStatus};

/// What the tray service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayCommand {
    /// Primary click on the item (SNI `Activate(x, y)`), for items without a menu.
    Activate { item: TrayItemId, x: i32, y: i32 },
    /// Middle click (`SecondaryActivate`).
    SecondaryActivate { item: TrayItemId, x: i32, y: i32 },
    /// Secondary click on an item with no dbusmenu (SNI `ContextMenu(x, y)`): the item shows its
    /// own menu at the screen point.
    ContextMenu { item: TrayItemId, x: i32, y: i32 },
    /// A menu row was picked (dbusmenu `Event(id, "clicked")`).
    MenuPick { item: TrayItemId, entry: i32 },
    /// The menu is about to show (dbusmenu `AboutToShow`), so the app can refresh it.
    MenuOpening { item: TrayItemId },
    /// The wheel turned over the item (SNI `Scroll(delta, orientation)`): a volume or
    /// brightness item steps with it.
    Scroll {
        item: TrayItemId,
        delta: ScrollDelta,
    },
}

/// How far the wheel turned over a status item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScrollDelta {
    pub axis: ScrollAxis,
    /// Whole wheel notches, sent as SNI's `delta` unchanged; the sign is `wl_pointer`'s
    /// (positive is down or right).
    pub steps: i32,
}

/// Which way a status item was scrolled (SNI's `orientation` argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

/// One change the host reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayEvent {
    /// A new item, or a re-registration of a known one (replaced in place).
    Registered(TrayItem),
    Title {
        item: TrayItemId,
        title: String,
    },
    Icon {
        item: TrayItemId,
        icon: TrayIcon,
    },
    Status {
        item: TrayItemId,
        status: TrayStatus,
    },
    Menu {
        item: TrayItemId,
        menu: Vec<TrayMenuEntry>,
    },
    Removed(TrayItemId),
}
