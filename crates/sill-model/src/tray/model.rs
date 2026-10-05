//! The tray's state: its items, their icons and their menu rows.

use serde::{Deserialize, Serialize};

use crate::units::{Availability, Visibility};

/// Every registered status item, in registration order (the bar draws them right to left
/// from the clock in this order).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tray {
    pub items: Vec<TrayItem>,
}

/// A status item's bus address: `<unique name><object path>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrayItemId(pub String);

/// One status notifier item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayItem {
    pub id: TrayItemId,
    /// `Title`, the tooltip and the accessible name.
    pub title: String,
    pub icon: TrayIcon,
    pub status: TrayStatus,
    /// The dbusmenu tree; empty when the item has no menu (or it has not arrived yet).
    pub menu: Vec<TrayMenuEntry>,
}

/// Where an item's icon comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayIcon {
    None,
    /// `IconName`, looked up in the icon theme.
    Named(String),
    /// `IconPixmap`, the largest one offered.
    Pixmap(Pixmap),
}

/// A raw ARGB32 icon, network byte order, as SNI sends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pixmap {
    pub width: u32,
    pub height: u32,
    pub argb32: Vec<u8>,
}

/// SNI `Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayStatus {
    /// Not important right now; the bar may hide it.
    Passive,
    #[default]
    Active,
    NeedsAttention,
}

/// One dbusmenu row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayMenuEntry {
    /// The dbusmenu id, sent back on a pick.
    pub id: i32,
    pub kind: TrayMenuKind,
}

/// A dbusmenu row's kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayMenuKind {
    Separator,
    Item {
        label: String,
        /// `enabled`; a disabled row is drawn at .35 and skipped by the keyboard.
        enabled: Availability,
        /// `visible`.
        visibility: Visibility,
        /// `toggle-type` + `toggle-state`.
        check: Check,
        /// `icon-name` or `icon-data`.
        icon: TrayMenuIcon,
        /// The submenu's rows; empty for a plain row.
        children: Vec<TrayMenuEntry>,
    },
}

/// The image at a dbusmenu row's start.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayMenuIcon {
    #[default]
    None,
    /// `icon-name`, looked up in the icon theme.
    Named(String),
    /// `icon-data`, PNG bytes.
    Png(Vec<u8>),
}

/// A row's check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    #[default]
    None,
    Unchecked,
    Checked,
    /// `toggle-state` indeterminate: some of what the row covers is on.
    Mixed,
}
