//! Where the desktop menu's popup goes: its top-left corner a pixel below and right of the
//! pointer, in the wallpaper's logical pixels, so the button's release at the same place is not over
//! the first entry (a release over an entry after a press outside the menu picks it). As the dock's menu, the popup is the menu plus transparent room for its
//! shadow ([`PopupRoom::CONTEXT`]) and shell-host's `PopupSize::FitContent` measures its
//! document; the compositor slides and flips it clear of the output's edges, so a press near the
//! right or bottom edge opens the menu leftwards or upwards.

use shell_host::{
    PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset, PopupPlacement,
    PopupSize, Rect, Size,
};
use sill_ui_kit::menu::{CONTEXT_MENU_WIDTH, PopupRoom};

use super::DesktopPoint;

/// The widest and tallest the popup grows: the menu, a submenu beside it, and their room.
const POPUP_MAX: i32 = 2 * CONTEXT_MENU_WIDTH + 8 + 2 * PopupRoom::CONTEXT.side;

/// How far the card's corner is from the pointer, on each axis.
const CLEAR: i32 = 1;

/// The popup's configuration: the anchor's top-left corner is the popup's, moved by the room
/// around the card so the card's corner is [`CLEAR`] from the pointer.
pub(super) fn menu_config() -> PopupConfig {
    PopupConfig {
        anchor: PopupAnchor::Rect,
        size: PopupSize::FitContent { max: POPUP_MAX },
        edge: PopupEdge::TopLeft,
        gravity: PopupEdge::BottomRight,
        adjust: PopupAdjust::SLIDE_AND_FLIP,
        grab: PopupGrab::Grabbed,
        offset: PopupOffset {
            dx: CLEAR - PopupRoom::CONTEXT.side,
            dy: CLEAR - PopupRoom::CONTEXT.top,
        },
    }
}

/// The placement at `at`: a one-pixel anchor on the pointer. The size is only where the popup
/// starts, before its document is measured: room for the menu and a submenu.
pub(super) fn menu_placement(at: DesktopPoint) -> PopupPlacement {
    PopupPlacement {
        anchor_rect: Rect {
            x: at.x,
            y: at.y,
            width: 1,
            height: 1,
        },
        size: Size {
            width: POPUP_MAX,
            height: POPUP_MAX,
        },
    }
}

#[cfg(test)]
mod tests {
    use shell_host::offset_rect;

    use super::DesktopPoint;
    use super::{CLEAR, menu_config, menu_placement};
    use sill_ui_kit::menu::PopupRoom;

    #[test]
    fn the_menus_corner_is_just_clear_of_the_pointer() {
        let at = DesktopPoint { x: 640, y: 300 };
        let placement = menu_placement(at);
        let config = menu_config();
        let anchor = offset_rect(placement.anchor_rect, config.offset);
        // Edge TopLeft puts the anchor point at the moved rect's top-left; gravity BottomRight
        // puts the popup's top-left corner on it; the card is the room inside the popup.
        assert_eq!(
            (
                anchor.x + PopupRoom::CONTEXT.side,
                anchor.y + PopupRoom::CONTEXT.top
            ),
            (at.x + CLEAR, at.y + CLEAR),
            "the card's corner"
        );
    }
}
