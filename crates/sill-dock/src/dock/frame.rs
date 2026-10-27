//! One frame of the dock, pure: where every tile, the pill, the Trash separator and the hit
//! element are, from the tiles, the settings and the machine's displayed progress and pointer
//! x (design/10 §10.3.1, §10.3.2, §10.3.12, §10.5.2). Surface-local and top-down: the layer is
//! the output's width and `gut + pad + S + T` tall, the tiles' bottoms on the baseline
//! `gut + pad` above its bottom edge.

use crate::magnify::{Layout, MagnifyParams, magnify, rest_centers};
use sill_model::units::{Cpx, Permille};
use sill_settings::DockSettings;

use crate::dock::geometry::magnify_params;
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::layer::GUTTER_PX;
use crate::dock::tokens::DockTokens;

/// The Trash separator: 1 px wide with 6 px either side (design/10 §10.3.2).
const SEPARATOR_BLOCK: Cpx = Cpx::px(13);

/// A rectangle in the surface, hundredths of a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Area {
    pub left: Cpx,
    pub top: Cpx,
    pub width: Cpx,
    pub height: Cpx,
}

impl Area {
    /// Whether `(x, y)` is inside.
    pub fn contains(&self, x: Cpx, y: Cpx) -> bool {
        x >= self.left
            && x < Cpx(self.left.0 + self.width.0)
            && y >= self.top
            && y < Cpx(self.top.0 + self.height.0)
    }

    /// This area grown by `by` on every side.
    pub fn inflate(&self, by: Cpx) -> Area {
        Area {
            left: Cpx(self.left.0 - by.0),
            top: Cpx(self.top.0 - by.0),
            width: Cpx(self.width.0 + 2 * by.0),
            height: Cpx(self.height.0 + 2 * by.0),
        }
    }

    /// The right edge.
    pub fn right(&self) -> Cpx {
        Cpx(self.left.0 + self.width.0)
    }
}

/// The dock's fixed vertical geometry for a tile size and a magnified size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rows {
    /// The surface's height.
    pub surface: Cpx,
    /// Where the tiles' bottoms rest.
    pub baseline: Cpx,
    /// The pill's top edge and height (`T + 2 pad`).
    pub pill_top: Cpx,
    pub pill_height: Cpx,
}

impl Rows {
    /// The rows for `dock`'s sizes and the pill's padding token.
    pub fn new(dock: &DockSettings, tokens: &DockTokens) -> Rows {
        let px = |v: u32| Cpx::px(i32::try_from(v).unwrap_or(i32::MAX / 100));
        let tile = u32::from(dock.tile_size_px.0);
        let magnified = u32::from(dock.magnified_size_px.0);
        let pad = u32::from(tokens.pad);
        let surface = GUTTER_PX + pad + magnified + tile;
        let pill_height = tile + 2 * pad;
        Rows {
            surface: px(surface),
            baseline: px(surface - GUTTER_PX - pad),
            pill_top: px(surface - GUTTER_PX - pill_height),
            pill_height: px(pill_height),
        }
    }
}

/// Everything one frame draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// One area per tile, in item order: magnified, on the baseline, not lifted.
    pub tiles: Vec<Area>,
    pub pill: Area,
    /// The separator's centre x, when there is a Trash tile after other tiles.
    pub separator: Option<Cpx>,
    /// The input element: the pill down to the output edge, grown over every magnified tile.
    pub hit: Area,
    /// Each tile's rest slot centre, which drops and drags count against.
    pub centers: Vec<Cpx>,
}

/// The rest centres of `items`, the Trash tile set apart by the separator block and the whole
/// row re-centred.
pub fn centers(items: &[DockItem], p: &MagnifyParams) -> Vec<Cpx> {
    let apart = items.len() > 1 && items.last().is_some_and(|i| i.kind == ItemKind::Trash);
    let shift = if apart { SEPARATOR_BLOCK.0 / 2 } else { 0 };
    let last = items.len().saturating_sub(1);
    rest_centers(items.len(), p)
        .into_iter()
        .enumerate()
        .map(|(i, c)| match (apart, i == last) {
            (true, true) => Cpx(c.0 - shift + SEPARATOR_BLOCK.0),
            _ => Cpx(c.0 - shift),
        })
        .collect()
}

/// The frame for `items` on an output `output_width_px` wide, at displayed `progress` with the
/// pointer at `x`.
pub fn frame(
    items: &[DockItem],
    dock: &DockSettings,
    tokens: &DockTokens,
    output_width_px: i32,
    progress: Permille,
    x: Cpx,
) -> Frame {
    let p = magnify_params(dock, tokens, output_width_px);
    let rows = Rows::new(dock, tokens);
    let centers = centers(items, &p);
    let layout = magnify(&centers, x, progress, &p);
    let tiles: Vec<Area> = layout
        .tiles
        .iter()
        .map(|t| Area {
            left: t.left,
            top: Cpx(rows.baseline.0 - t.size.0),
            width: t.size,
            height: t.size,
        })
        .collect();
    let pill = Area {
        left: layout.pill_left,
        top: rows.pill_top,
        width: Cpx(layout.pill_right.0 - layout.pill_left.0),
        height: rows.pill_height,
    };
    Frame {
        separator: separator(items, &layout),
        hit: hit_area(&pill, &tiles, rows.surface),
        tiles,
        pill,
        centers,
    }
}

fn separator(items: &[DockItem], layout: &Layout) -> Option<Cpx> {
    let n = items.len();
    if n < 2 || items[n - 1].kind != ItemKind::Trash {
        return None;
    }
    let before = layout.tiles[n - 2];
    let trash = layout.tiles[n - 1];
    Some(Cpx((before.left.0 + before.size.0 + trash.left.0) / 2))
}

fn hit_area(pill: &Area, tiles: &[Area], surface: Cpx) -> Area {
    let top = tiles
        .iter()
        .map(|t| t.top)
        .chain([pill.top])
        .min()
        .unwrap_or(pill.top);
    Area {
        left: pill.left,
        top,
        width: pill.width,
        height: Cpx(surface.0 - top.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{Rows, frame};
    use crate::dock::items::{DockItem, ItemKind};
    use crate::dock::machine::{TileApp, TileId};
    use crate::dock::tokens::DockTokens;
    use sill_launcher::AppId;
    use sill_model::units::{Cpx, Permille};
    use sill_settings::DockSettings;

    fn items(apps: usize, trash: bool) -> Vec<DockItem> {
        let mut items: Vec<DockItem> = (0..apps)
            .map(|i| DockItem {
                tile: TileId(u32::try_from(i).unwrap_or(0)),
                kind: ItemKind::Pinned(AppId(format!("app{i}"))),
                app: TileApp::NotRunning,
            })
            .collect();
        if trash {
            items.push(DockItem {
                tile: TileId(99),
                kind: ItemKind::Trash,
                app: TileApp::NotRunning,
            });
        }
        items
    }

    #[test]
    fn the_rows_are_design_10s_geometry() {
        let rows = Rows::new(&DockSettings::default(), &DockTokens::default());
        assert_eq!(rows.surface, Cpx::px(134));
        assert_eq!(rows.baseline, Cpx::px(120), "gut + pad above the bottom");
        assert_eq!(rows.pill_top, Cpx::px(66));
        assert_eq!(rows.pill_height, Cpx::px(60), "a tile and the 6 px padding");
    }

    #[test]
    fn at_rest_the_pill_is_w0_centred_and_the_hit_reaches_the_edge() {
        // Seven tiles: W0 = 7 x 48 + 6 x 8 + 12 = 396, centred on 1920: 762.
        let f = frame(
            &items(7, false),
            &DockSettings::default(),
            &DockTokens::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        assert_eq!((f.pill.left, f.pill.width), (Cpx::px(762), Cpx::px(396)));
        assert_eq!(f.separator, None);
        assert_eq!(f.hit.top, Cpx::px(66), "the pill's top at rest");
        assert_eq!(f.hit.height, Cpx::px(68), "pill plus gutter");
        assert!(f.tiles.iter().all(|t| t.width == Cpx::px(48)));
        assert_eq!(f.tiles[0].left, Cpx::px(768));
    }

    #[test]
    fn trash_widens_the_pill_by_the_separator_and_stays_centred() {
        let f = frame(
            &items(6, true),
            &DockSettings::default(),
            &DockTokens::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        assert_eq!(f.pill.width, Cpx::px(396 + 13));
        let centre = f.pill.left.0 + f.pill.width.0 / 2;
        assert!((centre - Cpx::px(960).0).abs() <= 100, "centre {centre}");
        let sep = f.separator.unwrap_or_default();
        assert!(sep > f.tiles[5].right() && sep < f.tiles[6].left, "{sep:?}");
    }

    #[test]
    fn a_magnified_tile_raises_the_hit_to_its_top() {
        let rest = frame(
            &items(7, false),
            &DockSettings::default(),
            &DockTokens::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        let x = Cpx(rest.tiles[3].left.0 + Cpx::px(24).0);
        let f = frame(
            &items(7, false),
            &DockSettings::default(),
            &DockTokens::default(),
            1920,
            Permille::ONE,
            x,
        );
        assert_eq!(f.tiles[3].width, Cpx::px(72));
        assert_eq!(f.hit.top, Cpx::px(120 - 72));
        assert!(f.tiles[3].contains(x, Cpx::px(100)), "the anchored tile");
    }

    /// design/10 §10.3.3 and R7: under the pointer the plate widens to hold the grown and slid
    /// tiles, it stays as tall as at rest and every tile's bottom stays on the baseline; the
    /// neighbours slide apart without overlapping; the tile under the pointer is the 72 px cap.
    #[test]
    fn magnifying_widens_the_plate_keeps_the_baseline_and_slides_the_neighbours_apart() {
        let items = items(6, true);
        let dock = DockSettings::default();
        let rest = frame(
            &items,
            &dock,
            &DockTokens::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        let under = rest.tiles[6].left.0 + Cpx::px(24).0;
        let near = frame(
            &items,
            &dock,
            &DockTokens::default(),
            1920,
            Permille::ONE,
            Cpx(under),
        );
        assert!(
            near.pill.width.0 > rest.pill.width.0 + Cpx::px(20).0,
            "the plate widens: {:?} to {:?}",
            rest.pill,
            near.pill
        );
        assert_eq!(
            (near.pill.top, near.pill.height),
            (rest.pill.top, rest.pill.height),
            "as tall as at rest"
        );
        let bottom = |a: &super::Area| a.top.0 + a.height.0;
        for (at, (now, was)) in near.tiles.iter().zip(&rest.tiles).enumerate() {
            assert_eq!(bottom(now), bottom(was), "tile {at} stays on the baseline");
        }
        assert_eq!(near.tiles[6].width, Cpx::px(72), "the cap");
        assert!(
            near.tiles
                .iter()
                .skip(1)
                .zip(&near.tiles)
                .all(|(next, before)| next.left >= before.right()),
            "no overlap: {:?}",
            near.tiles
        );
        assert!(
            near.tiles[0].left < rest.tiles[0].left,
            "the row spreads outward"
        );
        let held =
            near.tiles[0].left >= near.pill.left && near.tiles[6].right() <= near.pill.right();
        assert!(
            held,
            "every tile is on the plate: {:?} {:?}",
            near.pill, near.tiles
        );
    }
}
