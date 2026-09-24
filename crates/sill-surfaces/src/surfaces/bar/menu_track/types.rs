//! The menu tracker's vocabulary.

use shell_host::Point;
use sill_services::Stamp;
use sill_settings::MenusSettings;

/// A bar menu: one per title and status item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuId(pub u32);

/// An item, by its index at each nesting level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemPath(pub Vec<u16>);

/// Whether the button that opened the menu is still down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    Held,
    /// Released on the title: click mode.
    Released,
}

/// Whether the pointer has been inside the menu since it opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entered {
    Entered,
    NotYet,
}

/// Whether an item can be picked (separators, headers and disabled items cannot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Enabled,
    Inert,
}

/// Whether an item opens a submenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Leaf,
    Submenu,
}

/// What is under the pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Title(MenuId),
    Item {
        path: ItemPath,
        pick: Pick,
        branch: Branch,
    },
    /// Inside a menu, not on an item (its padding).
    Menu,
    Outside,
}

/// The safe triangle (design/13 §13.3.4): from the last pointer sample to the submenu's
/// near-edge corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guard {
    pub from: Point,
    pub top: Point,
    pub bottom: Point,
    /// When the pointer last moved inside the triangle.
    pub still_since: Stamp,
}

/// The open submenu, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sub {
    None,
    /// The pointer rests on `item`; its submenu opens after the delay.
    Pending {
        item: ItemPath,
        since: Stamp,
    },
    /// `guard` is `None` until the submenu's geometry is known.
    Open {
        item: ItemPath,
        guard: Option<Guard>,
    },
}

/// The tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuTrack {
    Closed,
    Tracking {
        menu: MenuId,
        held: Held,
        entered: Entered,
        /// The highlighted item (always a pickable one).
        hot: Option<ItemPath>,
        sub: Sub,
        /// The last pointer sample and what was under it.
        pointer: Point,
        under: Target,
    },
}

/// The keys that cross menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Escape,
    Left,
    Right,
    Enter,
}

/// One input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuIn {
    PressTitle(MenuId),
    Release(Target),
    Move(Point, Target),
    Key(MenuKey),
    /// A press outside every menu and title (`xdg_popup.popup_done`).
    OutsidePress,
    /// The open submenu's near-edge corners, once it is laid out.
    SubPlaced {
        top: Point,
        bottom: Point,
    },
    /// A timer asked for with `MenuOut::RequestTick` fired.
    Tick,
}

/// How a menu appears or leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anim {
    /// `menu-pop --t-move --e-spring`: only the first menu of a tracking session.
    MenuPop,
    /// `fade --t-quick --e-exit`.
    Fade,
}

/// Left or right along the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// One thing the bar must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuOut {
    Open(MenuId, Anim),
    /// Swap the open menu's content and reposition the one popup, same frame, no animation.
    Switch(MenuId),
    Close(Anim),
    OpenSub(ItemPath),
    CloseSub,
    Highlight(Option<ItemPath>),
    /// Run the item's action (after `Close`: design A6 "closes then picks").
    Pick(ItemPath),
    /// Open the next title's menu on this side (Left/Right with no submenu to act on).
    Adjacent(Side),
    RequestTick(Stamp),
}

/// The tracker's timings, from `menus.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuParams {
    pub submenu_delay_ms: u64,
    pub triangle_timeout_ms: u64,
}

impl MenuParams {
    /// From `menus.submenu_delay_ms` and `menus.submenu_triangle_timeout_ms`.
    pub fn new(menus: &MenusSettings) -> MenuParams {
        MenuParams {
            submenu_delay_ms: u64::from(menus.submenu_delay_ms.0),
            triangle_timeout_ms: u64::from(menus.submenu_triangle_timeout_ms.0),
        }
    }
}
