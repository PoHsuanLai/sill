//! Where a bar popup goes and how big it starts, in the bar surface's logical pixels.
//!
//! The popup is the menu plus room for its drop shadow: [`SIDE`] left and right, [`TOP`] above
//! and [`BOTTOM`] below, all transparent. The menu is a ds floating surface, which `ds::place`
//! keeps [`MARGIN`] inside its bounds (design/01 §8.2), so the popup's `Ds` root sits in a frame
//! inset by the room less that margin ([`frame_style`]) and the menu lands at (`SIDE`, `TOP`).
//! The anchor rectangle folds in the room; design/13 §13.3.2's offset from the title is the
//! config's (`menu_popup`).
//!
//! shell-host's `PopupSize::FitContent` never measures the popup's document (FINDINGS "M1
//! bar-ui", gap G7), so the size is estimated from the entries, generously: the popup's spare
//! area is transparent.

use ds::MenuEntry;
use shell_host::{PopupPlacement, Rect, Size};

use super::content::PopupBody;
use super::popup::MenuSide;

/// How far `ds::place` keeps a floating surface inside its bounds.
pub(crate) const MARGIN: i32 = 8;

/// The popup's transparent room around the menu, for `--shadow-pop` (`0 18px 40px -16px`):
/// 24 px each side, 6 px above (8 kept), 42 px below (40 kept, the rest is under 2 % alpha).
pub(crate) const SIDE: i32 = 24;
pub(crate) const TOP: i32 = 8;
pub(crate) const BOTTOM: i32 = 40;

/// The widest a bar menu grows (design/13 §13.3.3: max 420), with its room.
pub(crate) const MAX_PX: i32 = 420 + 2 * SIDE;

/// design/13 §13.3.3's minimum menu width, before the margin.
const MIN_WIDTH: i32 = 220;

/// Rough advance of one `--fs-control` (13 px) character, generous so labels never wrap.
const CHAR_PX: i32 = 8;

/// An item row: 6 px padding above and below a 17 px line, plus slack.
const ITEM_PX: i32 = 30;
/// A section header row.
const HEADER_PX: i32 = 28;
/// A separator: 1 px and 4 px above and below.
const SEPARATOR_PX: i32 = 9;
/// The menu's own padding, top and bottom together, plus the check column's width.
const CHROME_PX: i32 = 12;
const CHECK_COLUMN_PX: i32 = 22;

/// The volume panel's box, before the margin.
const VOLUME_PANEL: Size = Size {
    width: 260,
    height: 132,
};

/// The popup's placement for `body` under the element whose rect is `title`.
pub(crate) fn placement(title: ds::Rect, side: MenuSide, body: &PopupBody) -> PopupPlacement {
    PopupPlacement {
        anchor_rect: anchor_rect(title, side),
        size: popup_size(body),
    }
}

/// The anchor rectangle that, moved by `menu_popup`'s offset, puts the menu itself at design/13
/// §13.3.2's place: a title's menu at `title.left - 8`, a status item's right-aligned to
/// `item.right + 8`, both `title.bottom + 6`. The popup hangs from the anchor's bottom-left (a
/// title) or bottom-right (a status item), `menu_popup`'s edges.
pub(crate) fn anchor_rect(title: ds::Rect, side: MenuSide) -> Rect {
    let (left, top) = (floor(title.left()), floor(title.top()));
    let (right, bottom) = (ceil(title.right()), ceil(title.bottom()));
    let (left, right) = match side {
        MenuSide::Title => (left - SIDE, right),
        MenuSide::Status => (left, right + SIDE),
    };
    let bottom = bottom - TOP;
    Rect {
        x: left,
        y: top,
        width: (right - left).max(1),
        height: (bottom - top).max(1),
    }
}

/// The inline position of the frame the popup's `Ds` root sits in: inset so that `ds::place`'s
/// margin puts the menu at (`SIDE`, `TOP`).
pub(crate) fn frame_style() -> String {
    format!(
        "left:{side}px;right:{side}px;top:{top}px;bottom:{bottom}px",
        side = SIDE - MARGIN,
        top = TOP - MARGIN,
        bottom = BOTTOM - MARGIN
    )
}

/// The popup's size for `body`: the estimated menu plus its room, capped at [`MAX_PX`].
pub(crate) fn popup_size(body: &PopupBody) -> Size {
    let inner = match body {
        PopupBody::Menu(entries) => menu_size(entries),
        PopupBody::Volume(_) => VOLUME_PANEL,
    };
    Size {
        width: (inner.width + 2 * SIDE).min(MAX_PX),
        height: (inner.height + TOP + BOTTOM).min(MAX_PX),
    }
}

fn menu_size<T>(entries: &[MenuEntry<T>]) -> Size {
    let (longest, height) = entries.iter().fold((0, CHROME_PX), |(longest, h), entry| {
        let (chars, row) = match entry {
            MenuEntry::Item { title, .. } | MenuEntry::Submenu { title, .. } => {
                (title.chars().count(), ITEM_PX)
            }
            MenuEntry::Header(text) => (text.chars().count(), HEADER_PX),
            MenuEntry::Separator => (0, SEPARATOR_PX),
        };
        (longest.max(chars), h + row)
    });
    let text = i32::try_from(longest).unwrap_or(i32::MAX / CHAR_PX) * CHAR_PX;
    Size {
        width: (text + CHROME_PX + CHECK_COLUMN_PX + 2 * 9).max(MIN_WIDTH),
        height,
    }
}

fn floor(px: ds::Px) -> i32 {
    px.0.floor() as i32
}

fn ceil(px: ds::Px) -> i32 {
    px.0.ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::{BOTTOM, MAX_PX, SIDE, TOP, anchor_rect, popup_size};
    use crate::surfaces::bar::content::PopupBody;
    use crate::surfaces::bar::popup::{MenuSide, menu_popup};
    use ds::{MenuEntry, Point, Px, Rect as DsRect, Size as DsSize};
    use shell_host::Rect;
    use shell_host::popup::offset_rect;

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
            MenuSide::Status => (anchor.x + anchor.width - SIDE, y),
        }
    }

    #[test]
    fn menus_land_where_design_13_places_them() {
        // (title x, width, side, the menu's left (title) or right (status) edge, its top)
        const CASES: &[(f32, f32, MenuSide, i32, i32)] = &[
            (12.0, 60.0, MenuSide::Title, 4, 34),
            (1800.5, 110.2, MenuSide::Status, 1919, 34),
            (0.0, 28.0, MenuSide::Status, 36, 34),
        ];
        for &(x, width, side, edge, top) in CASES {
            let anchor = anchor_rect(title(x, width), side);
            assert_eq!(
                menu_corner(anchor, side),
                (edge, top),
                "{x} {width} {side:?}"
            );
        }
    }

    #[test]
    fn a_popup_is_its_menu_plus_the_margin_capped_at_the_maximum() {
        let short = PopupBody::Menu(vec![MenuEntry::Header("Clock".to_owned())]);
        let size = popup_size(&short);
        assert_eq!(size.width, 220 + 2 * SIDE, "the minimum width");
        assert!(size.height >= 28 + TOP + BOTTOM, "{size:?}");
        let long = PopupBody::Menu(
            (0..40)
                .map(|i| MenuEntry::Header(format!("{i:0>80}")))
                .collect(),
        );
        let size = popup_size(&long);
        assert_eq!((size.width, size.height), (MAX_PX, MAX_PX));
    }
}
