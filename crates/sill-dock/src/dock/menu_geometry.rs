//! Where the context menu's popup goes (design/10 §10.3.7: `Menu{Context}` anchored to its
//! tile, placement Top, gap 8), in the dock surface's logical pixels. The popup's geometry is
//! [`PopupGeom::OverTile`]; this is the tile it hangs over.

use shell_host::Rect;
use sill_model::units::Cpx;

use crate::dock::frame::Area;
use sill_ui_kit::menu::PopupGeom;

/// design/10 §10.3.7: the menu's gap above its tile.
pub(crate) const GEOM: PopupGeom = PopupGeom::OverTile { gap: 8 };

/// The anchor over the pill at `x` (hundredths of a pixel): a narrow strip as tall as the pill,
/// for the menu that belongs to no tile.
pub(crate) fn pill_anchor(pill: Area, x: Cpx) -> Rect {
    Rect {
        x: x.0.div_euclid(100) - 1,
        width: 2,
        ..tile_anchor(pill)
    }
}

/// The anchor over the tile drawn at `tile` (hundredths of a pixel), rounded outwards.
pub(crate) fn tile_anchor(tile: Area) -> Rect {
    let floor = |v: i32| v.div_euclid(100);
    let ceil = |v: i32| (v + 99).div_euclid(100);
    let (left, top) = (floor(tile.left.0), floor(tile.top.0));
    let (right, bottom) = (ceil(tile.right().0), ceil(tile.top.0 + tile.height.0));
    Rect {
        x: left,
        y: top,
        width: (right - left).max(1),
        height: (bottom - top).max(1),
    }
}
