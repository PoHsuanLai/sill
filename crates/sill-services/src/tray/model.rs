//! The tray's state, its commands and its one reducer.

use serde::{Deserialize, Serialize};

use crate::units::Visibility;

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
        children: Vec<TrayMenuEntry>,
    },
}

/// Whether a row can be picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    #[default]
    Enabled,
    Disabled,
}

/// A row's check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    #[default]
    None,
    Unchecked,
    Checked,
}

/// What the tray service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum TrayCommand {
    /// Primary click on the item (SNI `Activate(x, y)`), for items without a menu.
    Activate { item: TrayItemId, x: i32, y: i32 },
    /// Middle click (`SecondaryActivate`).
    SecondaryActivate { item: TrayItemId, x: i32, y: i32 },
    /// A menu row was picked (dbusmenu `Event(id, "clicked")`).
    MenuPick { item: TrayItemId, entry: i32 },
    /// The menu is about to show (dbusmenu `AboutToShow`), so the app can refresh it.
    MenuOpening { item: TrayItemId },
    /// The wheel turned over the item (SNI `Scroll(delta, orientation)`): a volume or
    /// brightness item steps with it. M2 freeze amendment (FINDINGS F54).
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

/// `tray` after `event`. A change for an unknown item is dropped (it raced its removal).
pub fn reduce_tray(tray: Tray, event: TrayEvent) -> Tray {
    let Tray { mut items } = tray;
    match event {
        TrayEvent::Registered(item) => match items.iter_mut().find(|i| i.id == item.id) {
            Some(known) => *known = item,
            None => items.push(item),
        },
        TrayEvent::Title { item, title } => edit(&mut items, &item, |i| i.title = title),
        TrayEvent::Icon { item, icon } => edit(&mut items, &item, |i| i.icon = icon),
        TrayEvent::Status { item, status } => edit(&mut items, &item, |i| i.status = status),
        TrayEvent::Menu { item, menu } => edit(&mut items, &item, |i| i.menu = menu),
        TrayEvent::Removed(item) => items.retain(|i| i.id != item),
    }
    Tray { items }
}

fn edit(items: &mut [TrayItem], id: &TrayItemId, change: impl FnOnce(&mut TrayItem)) {
    if let Some(item) = items.iter_mut().find(|i| &i.id == id) {
        change(item);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Availability, Check, Tray, TrayEvent, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry,
        TrayMenuKind, TrayStatus, reduce_tray,
    };
    use crate::units::Visibility;

    fn id(name: &str) -> TrayItemId {
        TrayItemId(format!(":1.{name}/StatusNotifierItem"))
    }

    fn item(name: &str) -> TrayItem {
        TrayItem {
            id: id(name),
            title: name.to_owned(),
            icon: TrayIcon::Named(format!("{name}-symbolic")),
            status: TrayStatus::Active,
            menu: Vec::new(),
        }
    }

    fn tray(names: &[&str]) -> Tray {
        Tray {
            items: names.iter().map(|n| item(n)).collect(),
        }
    }

    fn quit_menu() -> Vec<TrayMenuEntry> {
        vec![TrayMenuEntry {
            id: 7,
            kind: TrayMenuKind::Item {
                label: "Quit".to_owned(),
                enabled: Availability::Enabled,
                visibility: Visibility::Shown,
                check: Check::None,
                children: Vec::new(),
            },
        }]
    }

    #[test]
    fn events_reduce_to_the_expected_tray() {
        let with = |name: &str, edit: fn(&mut TrayItem)| {
            let mut t = tray(&["a", "b"]);
            if let Some(i) = t.items.iter_mut().find(|i| i.title == name) {
                edit(i);
            }
            t
        };
        let cases: Vec<(&str, Tray, TrayEvent, Tray)> = vec![
            (
                "registration appends",
                tray(&["a"]),
                TrayEvent::Registered(item("b")),
                tray(&["a", "b"]),
            ),
            (
                "re-registration replaces in place",
                tray(&["a", "b"]),
                TrayEvent::Registered(TrayItem {
                    status: TrayStatus::NeedsAttention,
                    ..item("a")
                }),
                with("a", |i| i.status = TrayStatus::NeedsAttention),
            ),
            (
                "title",
                tray(&["a", "b"]),
                TrayEvent::Title {
                    item: id("b"),
                    title: "B!".to_owned(),
                },
                {
                    let mut t = tray(&["a", "b"]);
                    t.items[1].title = "B!".to_owned();
                    t
                },
            ),
            (
                "icon",
                tray(&["a", "b"]),
                TrayEvent::Icon {
                    item: id("a"),
                    icon: TrayIcon::None,
                },
                with("a", |i| i.icon = TrayIcon::None),
            ),
            (
                "status",
                tray(&["a", "b"]),
                TrayEvent::Status {
                    item: id("b"),
                    status: TrayStatus::Passive,
                },
                with("b", |i| i.status = TrayStatus::Passive),
            ),
            (
                "menu",
                tray(&["a", "b"]),
                TrayEvent::Menu {
                    item: id("a"),
                    menu: quit_menu(),
                },
                with("a", |i| i.menu = quit_menu()),
            ),
            (
                "removal",
                tray(&["a", "b"]),
                TrayEvent::Removed(id("a")),
                tray(&["b"]),
            ),
            (
                "a change for an unknown item is dropped",
                tray(&["a"]),
                TrayEvent::Title {
                    item: id("zz"),
                    title: "x".to_owned(),
                },
                tray(&["a"]),
            ),
            (
                "removing an unknown item changes nothing",
                tray(&["a"]),
                TrayEvent::Removed(id("zz")),
                tray(&["a"]),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_tray(before, event), want, "{name}");
        }
    }

    #[test]
    fn a_scroll_has_a_pinned_wire_line() {
        use super::{ScrollAxis, ScrollDelta, TrayCommand};
        let command = TrayCommand::Scroll {
            item: id("vol"),
            delta: ScrollDelta {
                axis: ScrollAxis::Vertical,
                steps: -2,
            },
        };
        let line = r#"{"kind":"scroll","v":{"item":":1.vol/StatusNotifierItem","delta":{"axis":"vertical","steps":-2}}}"#;
        assert_eq!(serde_json::to_string(&command).ok().as_deref(), Some(line));
        let back: TrayCommand = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, command);
    }
}
