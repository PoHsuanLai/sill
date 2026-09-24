//! Where the context menu's popup goes and how big it is (design/10 §10.3.7: `Menu{Context}`
//! anchored to its tile, placement Top, gap 8), in the dock surface's logical pixels.
//!
//! As the bar's popups (FINDINGS F45, F53), the popup is the menu plus transparent room for its
//! shadow: [`SIDE`] each side, [`TOP`] above, [`BOTTOM`] below. The popup hangs above its
//! anchor (edge and gravity Top, centred), moved down by the room below the menu less the gap,
//! so the menu's bottom edge is [`GAP`] above the tile. The height is estimated from the
//! entries (wrapped titles included), generously; a menu that turns out shorter leaves
//! transparent room above it. shell-host's `PopupSize::FitContent` now measures a popup's
//! document (G7 closed), but the menu is an overlay placed from the popup's bottom edge by the
//! size given here, so a size the host changed afterwards would move the menu off its tile. With an `Options` submenu the popup is wide
//! enough for the submenu to open to the menu's right, and shifted so the menu itself stays
//! centred over the tile.

use ds::MenuEntry;
use shell_host::{
    Adjust, Edge, Grab, Offset, PopupAnchor, PopupConfig, PopupPlacement, PopupSize, Rect, Size,
    Toggle,
};

use crate::surfaces::dock::frame::Area;

/// How far `ds::place` keeps a floating surface inside its bounds.
pub(crate) const MARGIN: i32 = 8;
/// The popup's room around the menu, for `--shadow-pop`.
pub(crate) const SIDE: i32 = 24;
pub(crate) const TOP: i32 = 8;
pub(crate) const BOTTOM: i32 = 40;
/// design/10 §10.3.7: the menu's gap above its tile.
pub(crate) const GAP: i32 = 8;

/// A context menu's width (quire `menu.css`: Slim and Context are 220) and its cap.
pub(crate) const MENU_W: i32 = 220;
const MENU_MAX_H: i32 = 320;
/// The room a submenu needs beside the menu.
const SUBMENU_W: i32 = MENU_W + MARGIN;

/// Rows of a Context menu: an item's 6 px padding around a 22 px tile, a separator's 1 px and
/// 4 px margins, a header's line; the panel's 5 px padding top and bottom.
const ITEM_PX: i32 = 36;
const SEPARATOR_PX: i32 = 9;
const HEADER_PX: i32 = 30;
const PANEL_PX: i32 = 12;
/// A wrapped title's further lines: 13 px text at 1.55, about 20 characters to the line in the
/// Context menu's text column.
const LINE_PX: i32 = 20;
const LINE_CHARS: usize = 20;

/// How many lines `title` wraps to.
fn lines(title: &str) -> i32 {
    i32::try_from(title.chars().count().div_ceil(LINE_CHARS).max(1)).unwrap_or(1)
}

/// Whether the menu has a submenu to leave room for.
pub(crate) fn has_submenu<T>(entries: &[MenuEntry<T>]) -> bool {
    entries
        .iter()
        .any(|e| matches!(e, MenuEntry::Submenu { .. }))
}

/// The menu's estimated height, capped where the menu scrolls.
fn menu_height<T>(entries: &[MenuEntry<T>]) -> i32 {
    let rows: i32 = entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Item { title, .. } | MenuEntry::Submenu { title, .. } => {
                ITEM_PX + LINE_PX * (lines(title) - 1)
            }
            MenuEntry::Header(_) => HEADER_PX,
            MenuEntry::Separator => SEPARATOR_PX,
        })
        .sum();
    (rows + PANEL_PX).min(MENU_MAX_H)
}

/// The popup's size for `entries`.
pub(crate) fn popup_size<T>(entries: &[MenuEntry<T>]) -> Size {
    let extra = if has_submenu(entries) { SUBMENU_W } else { 0 };
    Size {
        width: MENU_W + extra + 2 * SIDE,
        height: menu_height(entries) + TOP + BOTTOM,
    }
}

/// The popup's configuration: above `tile`, centred on it (the menu itself, when a submenu
/// widens the popup to the right).
pub(crate) fn menu_config<T>(entries: &[MenuEntry<T>]) -> PopupConfig {
    let dx = if has_submenu(entries) {
        SUBMENU_W / 2
    } else {
        0
    };
    PopupConfig {
        anchor: PopupAnchor::Rect,
        size: PopupSize::Fixed,
        edge: Edge::Top,
        gravity: Edge::Top,
        adjust: Adjust {
            slide_x: Toggle::On,
            ..Adjust::NONE
        },
        grab: Grab::Grabbed,
        offset: Offset {
            dx,
            dy: BOTTOM - GAP,
        },
    }
}

/// The placement for `entries` over the tile drawn at `tile` (hundredths of a pixel).
pub(crate) fn menu_placement<T>(tile: Area, entries: &[MenuEntry<T>]) -> PopupPlacement {
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
        size: popup_size(entries),
    }
}

/// The inline inset of the frame the popup's `Ds` root sits in: `ds::place`'s margin then puts
/// the menu's left edge at [`SIDE`] and its bottom at [`BOTTOM`] from the popup's.
pub(crate) fn frame_style() -> String {
    format!(
        "left:{side}px;right:{side}px;top:{top}px;bottom:{bottom}px",
        side = SIDE - MARGIN,
        top = TOP - MARGIN,
        bottom = BOTTOM - MARGIN
    )
}

/// The point the Context menu is anchored at, in the popup: its left edge and its bottom edge
/// (a Context menu opens below its point and flips above it when there is no room below,
/// which there never is).
pub(crate) fn menu_point(size: Size) -> (f32, f32) {
    (to_f32(SIDE), to_f32(size.height - BOTTOM))
}

fn to_f32(v: i32) -> f32 {
    f32::from(i16::try_from(v).unwrap_or(i16::MAX))
}

#[cfg(test)]
mod tests {
    use super::{BOTTOM, GAP, SIDE, TOP, menu_config, menu_placement, popup_size};
    use crate::surfaces::dock::frame::Area;
    use crate::surfaces::dock::pick::DockPick;
    use ds::{Availability, MenuEntry, Trail};
    use shell_host::popup::offset_rect;
    use sill_launcher::WindowKey;
    use sill_services::Cpx;

    fn item() -> MenuEntry<DockPick> {
        MenuEntry::Item {
            value: DockPick::Activate(WindowKey(1)),
            title: "x".to_owned(),
            detail: None,
            tile: None,
            trail: Trail::None,
            check: None,
            availability: Availability::Enabled,
        }
    }

    #[test]
    fn the_menu_sits_8_px_above_its_tile_centred() {
        let tile = Area {
            left: Cpx::px(900),
            top: Cpx::px(96),
            width: Cpx::px(48),
            height: Cpx::px(48),
        };
        let entries = vec![item(), MenuEntry::Separator, item()];
        let placement = menu_placement(tile, &entries);
        let config = menu_config(&entries);
        let anchor = offset_rect(placement.anchor_rect, config.offset);
        // Edge and gravity Top: the popup's bottom is the moved anchor's top, centred on it.
        let popup_bottom = anchor.y;
        let menu_bottom = popup_bottom - BOTTOM;
        assert_eq!(menu_bottom, 96 - GAP);
        let popup_left = anchor.x + anchor.width / 2 - placement.size.width / 2;
        let menu_centre = popup_left + SIDE + 110;
        assert_eq!(menu_centre, 924, "the tile's centre");
        assert_eq!(placement.size.width, 220 + 2 * SIDE);
        assert_eq!(placement.size.height, 36 * 2 + 9 + 12 + TOP + BOTTOM);
    }

    #[test]
    fn a_submenu_widens_the_popup_and_the_menu_stays_centred() {
        let tile = Area {
            left: Cpx::px(900),
            top: Cpx::px(96),
            width: Cpx::px(48),
            height: Cpx::px(48),
        };
        let entries = vec![MenuEntry::Submenu {
            title: "Options".to_owned(),
            tile: None,
            availability: Availability::Enabled,
            children: vec![item()],
        }];
        let placement = menu_placement(tile, &entries);
        let anchor = offset_rect(placement.anchor_rect, menu_config(&entries).offset);
        let popup_left = anchor.x + anchor.width / 2 - placement.size.width / 2;
        assert_eq!(popup_left + SIDE + 110, 924);
        assert!(popup_size(&entries).width > 2 * 220 + 2 * SIDE);
    }
}
