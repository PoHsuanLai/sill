//! Where the window menu's popup goes: its card's top left corner on the point the compositor
//! named, in the desktop's logical pixels turned into the layer's (the layer covers its output).
//! As the desktop menu, the popup is the menu plus transparent room for its shadow
//! ([`PopupRoom::CONTEXT`]); shell-host's `PopupSize::FitContent` measures its document and the
//! compositor slides and flips it clear of the output's edges.

use shell_host::{
    PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset, PopupPlacement,
    PopupSize, Rect, Size,
};
use sill_ui_kit::menu::{CONTEXT_MENU_WIDTH, PopupRoom};

/// The widest and tallest the popup grows: the menu, a submenu beside it, and their room.
const POPUP_MAX: i32 = 2 * CONTEXT_MENU_WIDTH + 8 + 2 * PopupRoom::CONTEXT.side;

/// The popup's configuration: the anchor's top left corner is the popup's, moved by the room
/// around the card so the card's corner is on the anchor.
pub(super) fn menu_config() -> PopupConfig {
    PopupConfig {
        anchor: PopupAnchor::Rect,
        size: PopupSize::FitContent { max: POPUP_MAX },
        edge: PopupEdge::TopLeft,
        gravity: PopupEdge::BottomRight,
        adjust: PopupAdjust::SLIDE_AND_FLIP,
        grab: PopupGrab::Grabbed,
        offset: PopupOffset {
            dx: -PopupRoom::CONTEXT.side,
            dy: -PopupRoom::CONTEXT.top,
        },
    }
}

/// The placement at the layer's `local` point: a one-pixel anchor there. The size is only
/// where the popup starts, before its document is measured.
pub(super) fn menu_placement(local: (i32, i32)) -> PopupPlacement {
    PopupPlacement {
        anchor_rect: Rect {
            x: local.0,
            y: local.1,
            width: 1,
            height: 1,
        },
        size: Size {
            width: POPUP_MAX,
            height: POPUP_MAX,
        },
    }
}

/// `at`, in the desktop's logical pixels, in the layer of the output whose rectangle is
/// `output`; `None` when the point is on another output.
pub(super) fn local_point(at: (i32, i32), output: Rect) -> Option<(i32, i32)> {
    let inside_x = at.0 >= output.x && at.0 < output.x.saturating_add(output.width);
    let inside_y = at.1 >= output.y && at.1 < output.y.saturating_add(output.height);
    (inside_x && inside_y).then(|| (at.0 - output.x, at.1 - output.y))
}

#[cfg(test)]
mod tests {
    use shell_host::{Rect, offset_rect};

    use super::{local_point, menu_config, menu_placement};
    use sill_ui_kit::menu::PopupRoom;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn the_points_on_the_output_are_made_local_and_the_rest_are_not_ours() {
        let right = rect(1920, 0, 1920, 1080);
        // (case, the point, what the right-hand output makes of it)
        let cases: [(&str, (i32, i32), Option<(i32, i32)>); 5] = [
            ("its top left corner", (1920, 0), Some((0, 0))),
            ("inside", (2000, 40), Some((80, 40))),
            ("past its right edge", (3840, 40), None),
            ("on the other output", (100, 40), None),
            ("past its bottom edge", (2000, 1080), None),
        ];
        for (name, at, want) in cases {
            assert_eq!(local_point(at, right), want, "{name}");
        }
    }

    #[test]
    fn the_cards_corner_is_on_the_point() {
        let placement = menu_placement((640, 300));
        let config = menu_config();
        let anchor = offset_rect(placement.anchor_rect, config.offset);
        // Edge TopLeft puts the anchor point at the moved rect's top left; gravity BottomRight
        // puts the popup's top left corner on it; the card is the room inside the popup.
        assert_eq!(
            (
                anchor.x + PopupRoom::CONTEXT.side,
                anchor.y + PopupRoom::CONTEXT.top
            ),
            (640, 300)
        );
    }
}
