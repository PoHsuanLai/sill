//! The bar's menu popup (design/20 §1.1: `open_popup(anchor Element(btn), FitContent, grab
//! Seat)`; design/13 §13.3.2 position: `x = title.left - 8`, `y = title.bottom + 6`, status
//! menus right-aligned to `item.right + 8`, one reusable popup repositioned on hover switch).

use shell_host::{Adjust, Edge, ElementId, Grab, PopupAnchor, PopupConfig, PopupSize};

/// Which side of its title a menu hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSide {
    /// Left-side titles: the menu's left edge 8 px left of the title's.
    Title,
    /// Right-side status items: the menu's right edge 8 px right of the item's.
    Status,
}

/// The popup configuration for a bar menu anchored to the element `button`, at most `max_px`
/// in each direction.
pub fn menu_popup(button: &str, side: MenuSide, max_px: i32) -> PopupConfig {
    let (edge, gravity) = match side {
        MenuSide::Title => (Edge::BottomLeft, Edge::BottomRight),
        MenuSide::Status => (Edge::BottomRight, Edge::BottomLeft),
    };
    PopupConfig {
        anchor: PopupAnchor::Element(ElementId::new(button)),
        size: PopupSize::FitContent { max: max_px },
        edge,
        gravity,
        adjust: Adjust::SLIDE_AND_FLIP,
        grab: Grab::Grabbed,
    }
}

/// The offset from the anchor corner design/13 §13.3.2 places a menu at: (-8, +6) for a title,
/// (+8, +6) for a status item. shell-host's `PopupConfig` has no offset (FINDINGS.md, gap G3),
/// so the component inflates the anchor element by this much instead.
pub fn menu_offset(side: MenuSide) -> (i32, i32) {
    match side {
        MenuSide::Title => (-8, 6),
        MenuSide::Status => (8, 6),
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuSide, menu_offset, menu_popup};
    use shell_host::{Edge, ElementId, Grab, PopupAnchor, PopupSize};

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
        let status = menu_popup("bar-status-clock", MenuSide::Status, 600);
        assert_eq!(
            (status.edge, status.gravity),
            (Edge::BottomRight, Edge::BottomLeft)
        );
        assert_eq!(menu_offset(MenuSide::Title), (-8, 6));
        assert_eq!(menu_offset(MenuSide::Status), (8, 6));
    }
}
