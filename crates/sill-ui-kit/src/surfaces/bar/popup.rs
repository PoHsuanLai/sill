//! The bar's menu popup (design/20 §1.1: `open_popup(anchor Element(btn), FitContent, grab
//! Seat)`; design/13 §13.3.2 position: `x = title.left - 8`, `y = title.bottom + 6`, status
//! menus right-aligned to `item.right + 8`, one reusable popup repositioned on hover switch).

use shell_host::{
    ElementId, PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset,
    PopupPlacement, PopupSize, Rect, Size,
};

use crate::menu::PopupRoom;

/// The control center's gap to the output's right edge (the popup margin, as a status menu's
/// 8 past its item).
pub(crate) const EDGE_GAP: i32 = 8;

/// The popup's transparent room around the menu, for `--shadow-pop` (`0 18px 40px -16px`):
/// 28 px each side (the polish pass's ambient shadow reaches 28), 8 px above, 40 px below (the
/// rest is under 2 % alpha).
pub(crate) const SIDE: i32 = 28;
pub(crate) const TOP: i32 = 8;
pub(crate) const BOTTOM: i32 = 40;

/// The widest a bar text menu grows (design/13 §13.3.3: max 420), with its room.
pub(crate) const MAX_PX: i32 = 420 + 2 * SIDE;

/// The room the popup's frame leaves around the menu.
pub(crate) fn room() -> PopupRoom {
    PopupRoom {
        side: SIDE,
        top: TOP,
        bottom: BOTTOM,
    }
}

/// The popup's placement under the element whose rect is `title`. The size is only where the
/// popup starts, before shell-host measures its document (`PopupSize::FitContent`): room for
/// any menu, so the first frame clips nothing.
pub(crate) fn placement(title: ds::Rect, side: MenuSide, bar_width: i32) -> PopupPlacement {
    PopupPlacement {
        anchor_rect: anchor_rect(title, side, bar_width),
        size: Size {
            width: MAX_PX,
            height: MAX_PX,
        },
    }
}

/// The anchor rectangle that, moved by `menu_popup`'s offset, puts the menu itself at design/13
/// §13.3.2's place: a title's menu at `title.left - 8`, a status item's right-aligned to
/// `item.right + 8`, both `title.bottom + 6`; the control center's right-aligned to the bar's
/// right end less [`EDGE_GAP`], whatever its item's place (macOS's top right). The popup hangs
/// from the anchor's bottom-left (a title) or bottom-right (a status item, the control center),
/// `menu_popup`'s edges; the anchor folds in the room.
pub(crate) fn anchor_rect(title: ds::Rect, side: MenuSide, bar_width: i32) -> Rect {
    let (left, top) = (floor(title.left()), floor(title.top()));
    let (right, bottom) = (ceil(title.right()), ceil(title.bottom()));
    let (left, right) = match side {
        MenuSide::Title => (left - SIDE, right),
        MenuSide::Status => (left, right + SIDE),
        // The anchor is the bar's last pixel column: the offset carries the popup's room past
        // the edge (`menu_offset`), so the menu's right edge lands `EDGE_GAP` inside it.
        MenuSide::OutputEdge => (bar_width - 1, bar_width),
    };
    let bottom = bottom - TOP;
    Rect {
        x: left,
        y: top,
        width: (right - left).max(1),
        height: (bottom - top).max(1),
    }
}

fn floor(px: ds::Px) -> i32 {
    px.0.floor() as i32
}

fn ceil(px: ds::Px) -> i32 {
    px.0.ceil() as i32
}

/// Which side of its title a menu hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSide {
    /// Left-side titles: the menu's left edge 8 px left of the title's.
    Title,
    /// Right-side status items: the menu's right edge 8 px right of the item's.
    Status,
    /// The control center: the menu's right edge at the output's right edge, less the popup
    /// margin, whatever the item's place (macOS's top-right panel).
    OutputEdge,
}

/// The popup configuration for a bar menu anchored to the element `button`, fitted to its
/// content at most `max_px` in each direction, offset from the anchor corner by
/// [`menu_offset`].
pub fn menu_popup(button: &str, side: MenuSide, max_px: i32) -> PopupConfig {
    let (edge, gravity) = match side {
        MenuSide::Title => (PopupEdge::BottomLeft, PopupEdge::BottomRight),
        MenuSide::Status | MenuSide::OutputEdge => (PopupEdge::BottomRight, PopupEdge::BottomLeft),
    };
    PopupConfig {
        anchor: PopupAnchor::Element(ElementId::new(button)),
        size: PopupSize::FitContent { max: max_px },
        edge,
        gravity,
        adjust: adjust(side),
        grab: PopupGrab::Grabbed,
        offset: {
            let (dx, dy) = menu_offset(side);
            PopupOffset { dx, dy }
        },
    }
}

/// The offset from the anchor corner design/13 §13.3.2 places a menu at: (-8, +6) for a title,
/// (+8, +6) for a status item: `PopupConfig::offset`.
pub fn menu_offset(side: MenuSide) -> (i32, i32) {
    match side {
        MenuSide::Title => (-8, 6),
        MenuSide::Status => (8, 6),
        // The anchor is the bar's last column: past it by the room, less the edge gap.
        MenuSide::OutputEdge => (SIDE - EDGE_GAP, 6),
    }
}

/// How the compositor may move a menu that would leave the output: slide and flip, except the
/// control center: its popup's shadow room reaches past the output's right edge, and past the
/// bottom when the panel is at its cap, on purpose (the panel itself stays [`EDGE_GAP`] inside
/// and `control_center.bottom_margin_px` above), and sliding it back would pull the panel in by
/// the room.
pub fn adjust(side: MenuSide) -> PopupAdjust {
    match side {
        MenuSide::Title | MenuSide::Status => PopupAdjust::SLIDE_AND_FLIP,
        MenuSide::OutputEdge => PopupAdjust::NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PX, MenuSide, SIDE, TOP, anchor_rect, menu_offset, menu_popup};
    use ds::{Point, Px, Rect as DsRect, Size as DsSize};
    use shell_host::{
        ElementId, PopupAnchor, PopupEdge, PopupGrab, PopupOffset, PopupSize, Rect, offset_rect,
    };

    fn title(x: f32, width: f32) -> DsRect {
        DsRect {
            origin: Point {
                x: Px(x),
                y: Px(4.0),
            },
            size: DsSize {
                width: Px(width),
                height: Px(24.0),
            },
        }
    }

    /// Where the menu itself lands: the offset anchor's corner (bottom-left for a title,
    /// bottom-right for a status item) moved in by the popup's room; for a status item the
    /// menu's right edge.
    fn menu_corner(anchor: Rect, side: MenuSide) -> (i32, i32) {
        let anchor = offset_rect(anchor, menu_popup("bar", side, MAX_PX).offset);
        let y = anchor.y + anchor.height + TOP;
        match side {
            MenuSide::Title => (anchor.x + SIDE, y),
            MenuSide::Status | MenuSide::OutputEdge => (anchor.x + anchor.width - SIDE, y),
        }
    }

    #[test]
    fn menus_land_where_design_13_places_them() {
        // (title x, width, side, the menu's left (title) or right (status) edge, its top)
        const CASES: &[(f32, f32, MenuSide, i32, i32)] = &[
            (12.0, 60.0, MenuSide::Title, 4, 34),
            (1800.5, 110.2, MenuSide::Status, 1919, 34),
            (0.0, 28.0, MenuSide::Status, 36, 34),
            // The control center: 8 inside the bar's right end, wherever its item is.
            (1700.0, 28.0, MenuSide::OutputEdge, 1912, 34),
            (100.0, 28.0, MenuSide::OutputEdge, 1912, 34),
        ];
        for &(x, width, side, edge, top) in CASES {
            let anchor = anchor_rect(title(x, width), side, 1920);
            assert_eq!(
                menu_corner(anchor, side),
                (edge, top),
                "{x} {width} {side:?}"
            );
        }
    }

    #[test]
    fn titles_hang_right_and_status_items_hang_left() {
        let title = menu_popup("bar-title-app", MenuSide::Title, 600);
        assert_eq!(
            title.anchor,
            PopupAnchor::Element(ElementId::new("bar-title-app"))
        );
        assert_eq!(
            (title.edge, title.gravity),
            (PopupEdge::BottomLeft, PopupEdge::BottomRight)
        );
        assert_eq!(title.size, PopupSize::FitContent { max: 600 });
        assert_eq!(title.grab, PopupGrab::Grabbed);
        assert_eq!(title.offset, PopupOffset { dx: -8, dy: 6 });
        let status = menu_popup("bar-status-clock", MenuSide::Status, 600);
        assert_eq!(
            (status.edge, status.gravity),
            (PopupEdge::BottomRight, PopupEdge::BottomLeft)
        );
        assert_eq!(status.offset, PopupOffset { dx: 8, dy: 6 });
        assert_eq!(menu_offset(MenuSide::Title), (-8, 6));
        assert_eq!(menu_offset(MenuSide::Status), (8, 6));
        let edge = menu_popup("bar-status-cc", MenuSide::OutputEdge, 600);
        assert_eq!(
            (edge.edge, edge.gravity),
            (PopupEdge::BottomRight, PopupEdge::BottomLeft)
        );
        assert_eq!(edge.offset, PopupOffset { dx: 20, dy: 6 });
        assert_eq!(edge.adjust, shell_host::PopupAdjust::NONE);
        assert_eq!(status.adjust, shell_host::PopupAdjust::SLIDE_AND_FLIP);
    }
}
