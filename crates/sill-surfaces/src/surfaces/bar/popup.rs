//! The bar's menu popup (design/20 §1.1: `open_popup(anchor Element(btn), FitContent, grab
//! Seat)`; design/13 §13.3.2 position: `x = title.left - 8`, `y = title.bottom + 6`, status
//! menus right-aligned to `item.right + 8`, one reusable popup repositioned on hover switch).

use shell_host::{Adjust, Edge, ElementId, Grab, Offset, PopupAnchor, PopupConfig, PopupSize};

use super::geometry::{EDGE_GAP, SIDE};

/// Which side of its title a menu hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSide {
    /// Left-side titles: the menu's left edge 8 px left of the title's.
    Title,
    /// Right-side status items: the menu's right edge 8 px right of the item's.
    Status,
    /// The control center: the menu's right edge at the output's right edge, less the popup
    /// margin, whatever the item's place (macOS's top-right panel; M5 agent c places it).
    OutputEdge,
}

/// The popup configuration for a bar menu anchored to the element `button`, at most `max_px`
/// in each direction, offset from the anchor corner by [`menu_offset`].
pub fn menu_popup(button: &str, side: MenuSide, max_px: i32) -> PopupConfig {
    let (edge, gravity) = match side {
        MenuSide::Title => (Edge::BottomLeft, Edge::BottomRight),
        MenuSide::Status | MenuSide::OutputEdge => (Edge::BottomRight, Edge::BottomLeft),
    };
    PopupConfig {
        anchor: PopupAnchor::Element(ElementId::new(button)),
        size: PopupSize::FitContent { max: max_px },
        edge,
        gravity,
        adjust: adjust(side),
        grab: Grab::Grabbed,
        offset: {
            let (dx, dy) = menu_offset(side);
            Offset { dx, dy }
        },
    }
}

/// The offset from the anchor corner design/13 §13.3.2 places a menu at: (-8, +6) for a title,
/// (+8, +6) for a status item: `PopupConfig::offset` (shell-host closed gap G3).
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
pub fn adjust(side: MenuSide) -> Adjust {
    match side {
        MenuSide::Title | MenuSide::Status => Adjust::SLIDE_AND_FLIP,
        MenuSide::OutputEdge => Adjust::NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuSide, menu_offset, menu_popup};
    use shell_host::{Edge, ElementId, Grab, Offset, PopupAnchor, PopupSize};

    #[test]
    fn titles_hang_right_and_status_items_hang_left() {
        let title = menu_popup("bar-title-app", MenuSide::Title, 600);
        assert_eq!(
            title.anchor,
            PopupAnchor::Element(ElementId::new("bar-title-app"))
        );
        assert_eq!(
            (title.edge, title.gravity),
            (Edge::BottomLeft, Edge::BottomRight)
        );
        assert_eq!(title.size, PopupSize::FitContent { max: 600 });
        assert_eq!(title.grab, Grab::Grabbed);
        assert_eq!(title.offset, Offset { dx: -8, dy: 6 });
        let status = menu_popup("bar-status-clock", MenuSide::Status, 600);
        assert_eq!(
            (status.edge, status.gravity),
            (Edge::BottomRight, Edge::BottomLeft)
        );
        assert_eq!(status.offset, Offset { dx: 8, dy: 6 });
        assert_eq!(menu_offset(MenuSide::Title), (-8, 6));
        assert_eq!(menu_offset(MenuSide::Status), (8, 6));
        let edge = menu_popup("bar-status-cc", MenuSide::OutputEdge, 600);
        assert_eq!(
            (edge.edge, edge.gravity),
            (Edge::BottomRight, Edge::BottomLeft)
        );
        assert_eq!(edge.offset, Offset { dx: 20, dy: 6 });
        assert_eq!(edge.adjust, shell_host::Adjust::NONE);
        assert_eq!(status.adjust, shell_host::Adjust::SLIDE_AND_FLIP);
    }
}
