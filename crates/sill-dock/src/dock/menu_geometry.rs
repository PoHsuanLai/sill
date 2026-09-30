//! Where the context menu's popup goes (design/10 §10.3.7: `Menu{Context}` anchored to its
//! tile, placement Top, gap 8), in the dock surface's logical pixels.
//!
//! As the bar's popups, the popup is the menu plus transparent room for its shadow ([`room`]),
//! and shell-host's `PopupSize::FitContent` measures the popup's document, so its size is the
//! menu's own. The popup hangs above its anchor from its bottom-left corner (edge Top, gravity
//! TopRight), moved left by half the menu's width and its room and down by the room below the
//! menu less the gap, so the menu is centred over the tile with its bottom edge [`GAP`] above
//! it; an `Options` submenu opening to the menu's right widens the popup rightwards and moves
//! nothing.

use shell_host::{
    PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset, PopupPlacement,
    PopupSize, PopupToggle, Rect, Size,
};

use crate::dock::frame::Area;
use sill_ui_kit::menu::PopupRoom;

/// The popup's room around the menu, for `--shadow-pop` (its ambient layer reaches 28 px).
pub(crate) const SIDE: i32 = 28;
pub(crate) const TOP: i32 = 8;
pub(crate) const BOTTOM: i32 = 40;
/// design/10 §10.3.7: the menu's gap above its tile.
pub(crate) const GAP: i32 = 8;

/// A Context menu's width: quire's `menu.css` fixes Slim and Context at 220. The menu is
/// centred over its tile by it.
pub(crate) const MENU_W: i32 = 220;

/// The widest and tallest the popup grows: the menu, a submenu beside it, and their room.
const POPUP_MAX: i32 = 2 * MENU_W + 8 + 2 * SIDE;

/// The popup's configuration: above the tile's top edge, its bottom-left corner half a menu
/// (and the room) left of the tile's centre.
pub(crate) fn menu_config() -> PopupConfig {
    PopupConfig {
        anchor: PopupAnchor::Rect,
        size: PopupSize::FitContent { max: POPUP_MAX },
        edge: PopupEdge::Top,
        gravity: PopupEdge::TopRight,
        adjust: PopupAdjust {
            slide_x: PopupToggle::On,
            ..PopupAdjust::NONE
        },
        grab: PopupGrab::Grabbed,
        offset: PopupOffset {
            dx: -(MENU_W / 2 + SIDE),
            dy: BOTTOM - GAP,
        },
    }
}

/// The placement over the tile drawn at `tile` (hundredths of a pixel). The size is only where
/// the popup starts, before its document is measured: room for the menu and a submenu.
pub(crate) fn menu_placement(tile: Area) -> PopupPlacement {
    let floor = |v: i32| v.div_euclid(100);
    let ceil = |v: i32| (v + 99).div_euclid(100);
    let (left, top) = (floor(tile.left.0), floor(tile.top.0));
    let (right, bottom) = (ceil(tile.right().0), ceil(tile.top.0 + tile.height.0));
    PopupPlacement {
        anchor_rect: Rect {
            x: left,
            y: top,
            width: (right - left).max(1),
            height: (bottom - top).max(1),
        },
        size: Size {
            width: POPUP_MAX,
            height: POPUP_MAX,
        },
    }
}

/// The room the popup's frame leaves around the menu.
pub(crate) fn room() -> PopupRoom {
    PopupRoom {
        side: SIDE,
        top: TOP,
        bottom: BOTTOM,
    }
}

#[cfg(test)]
mod tests {
    use super::{BOTTOM, GAP, MENU_W, SIDE, menu_config, menu_placement};

    use crate::dock::frame::Area;
    use shell_host::offset_rect;
    use sill_model::units::Cpx;

    #[test]
    fn the_menu_sits_8_px_above_its_tile_centred() {
        let tile = Area {
            left: Cpx::px(900),
            top: Cpx::px(96),
            width: Cpx::px(48),
            height: Cpx::px(48),
        };
        let placement = menu_placement(tile);
        let config = menu_config();
        let anchor = offset_rect(placement.anchor_rect, config.offset);
        // Edge Top puts the anchor point at the moved rect's top centre; gravity TopRight puts
        // the popup's bottom-left corner on it.
        let (popup_left, popup_bottom) = (anchor.x + anchor.width / 2, anchor.y);
        assert_eq!(popup_bottom - BOTTOM, 96 - GAP, "the menu's bottom edge");
        assert_eq!(popup_left + SIDE + MENU_W / 2, 924, "the tile's centre");
    }
}
