//! Where a context menu's popup goes. Every menu popup is the menu plus transparent room for
//! its shadow ([`PopupRoom::CONTEXT`]); shell-host's `PopupSize::FitContent` measures the
//! popup's document, so only the anchor and the way the popup hangs off it differ: at a pointer
//! or over a tile. The widest the popup grows is one number here, not one per surface.

use shell_host::{
    PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset, PopupPlacement,
    PopupSize, PopupToggle, Rect, Size,
};

use super::{CONTEXT_MENU_WIDTH, PopupRoom};

/// The widest and tallest the popup grows: the menu, a submenu beside it, and their room.
const POPUP_MAX: i32 = 2 * CONTEXT_MENU_WIDTH + 8 + 2 * PopupRoom::CONTEXT.side;

/// How a context menu's popup hangs off its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupGeom {
    /// The card's top left corner `clear` pixels right of and below the anchor point (the
    /// compositor slides and flips it clear of the output's edges). `clear` keeps the release
    /// of the press that opened it off the first entry.
    AtPoint { clear: i32 },
    /// Above the anchor tile, centred on it, the card's bottom edge `gap` above the tile.
    OverTile { gap: i32 },
}

/// A one-pixel anchor at `(x, y)`, for [`PopupGeom::AtPoint`].
pub fn point_anchor(x: i32, y: i32) -> Rect {
    Rect {
        x,
        y,
        width: 1,
        height: 1,
    }
}

impl PopupGeom {
    /// The popup's configuration.
    pub fn config(self) -> PopupConfig {
        let room = PopupRoom::CONTEXT;
        let (edge, gravity, adjust, offset) = match self {
            PopupGeom::AtPoint { clear } => (
                PopupEdge::TopLeft,
                PopupEdge::BottomRight,
                PopupAdjust::SLIDE_AND_FLIP,
                PopupOffset {
                    dx: clear - room.side,
                    dy: clear - room.top,
                },
            ),
            PopupGeom::OverTile { gap } => (
                PopupEdge::Top,
                PopupEdge::TopRight,
                PopupAdjust {
                    slide_x: PopupToggle::On,
                    ..PopupAdjust::NONE
                },
                PopupOffset {
                    dx: -(CONTEXT_MENU_WIDTH / 2 + room.side),
                    dy: room.bottom - gap,
                },
            ),
        };
        PopupConfig {
            anchor: PopupAnchor::Rect,
            size: PopupSize::FitContent { max: POPUP_MAX },
            edge,
            gravity,
            adjust,
            grab: PopupGrab::Grabbed,
            offset,
        }
    }

    /// The placement at `anchor`. The size is only where the popup starts, before its document
    /// is measured: room for the menu and a submenu.
    pub fn placement(self, anchor: Rect) -> PopupPlacement {
        PopupPlacement {
            anchor_rect: anchor,
            size: Size {
                width: POPUP_MAX,
                height: POPUP_MAX,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use shell_host::offset_rect;

    use super::{CONTEXT_MENU_WIDTH, PopupGeom, PopupRoom, Rect, point_anchor};

    #[test]
    fn the_card_lands_where_each_geometry_says() {
        let room = PopupRoom::CONTEXT;
        let tile = Rect {
            x: 900,
            y: 96,
            width: 48,
            height: 48,
        };
        for (geom, anchor) in [
            (PopupGeom::AtPoint { clear: 1 }, point_anchor(640, 300)),
            (PopupGeom::AtPoint { clear: 0 }, point_anchor(640, 300)),
            (PopupGeom::OverTile { gap: 8 }, tile),
        ] {
            let config = geom.config();
            let moved = offset_rect(geom.placement(anchor).anchor_rect, config.offset);
            match geom {
                // Edge TopLeft puts the anchor at the moved rect's top left; gravity
                // BottomRight puts the popup's top left corner on it.
                PopupGeom::AtPoint { clear } => assert_eq!(
                    (moved.x + room.side, moved.y + room.top),
                    (640 + clear, 300 + clear),
                    "{geom:?}: the card's corner"
                ),
                // Edge Top puts the anchor at the moved rect's top centre; gravity TopRight
                // puts the popup's bottom left corner on it.
                PopupGeom::OverTile { gap } => {
                    assert_eq!(moved.y - room.bottom, 96 - gap, "the card's bottom edge");
                    assert_eq!(
                        moved.x + moved.width / 2 + room.side + CONTEXT_MENU_WIDTH / 2,
                        924,
                        "the tile's centre"
                    );
                }
            }
        }
    }
}
