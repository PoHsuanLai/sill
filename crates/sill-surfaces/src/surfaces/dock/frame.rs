//! One frame of the dock, pure: where every tile, the pill, the Trash separator and the hit
//! element are, from the tiles, the settings and the machine's displayed progress and pointer
//! x (design/10 §10.3.1, §10.3.2, §10.3.12, §10.5.2). Surface-local and top-down: the layer is
//! the output's width and `gut + pad + S + T` tall, the tiles' bottoms on the baseline
//! `gut + pad` above its bottom edge.

use sill_services::{Cpx, Layout, MagnifyParams, Permille, magnify, rest_centers};
use sill_settings::DockSettings;

use crate::surfaces::dock::geometry::magnify_params;
use crate::surfaces::dock::items::{DockItem, ItemKind};
use crate::surfaces::dock::layer::{GUTTER_PX, PAD_PX};

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
    /// The rows for `dock`'s sizes.
    pub fn new(dock: &DockSettings) -> Rows {
        let px = |v: u32| Cpx::px(i32::try_from(v).unwrap_or(i32::MAX / 100));
        let tile = u32::from(dock.tile_size_px.0);
        let magnified = u32::from(dock.magnified_size_px.0);
        let surface = GUTTER_PX + PAD_PX + magnified + tile;
        let pill_height = tile + 2 * PAD_PX;
        Rows {
            surface: px(surface),
            baseline: px(surface - GUTTER_PX - PAD_PX),
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
    output_width_px: i32,
    progress: Permille,
    x: Cpx,
) -> Frame {
    let p = magnify_params(dock, output_width_px);
    let rows = Rows::new(dock);
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
    use crate::surfaces::dock::items::{DockItem, ItemKind};
    use crate::surfaces::dock::machine::{TileApp, TileId};
    use sill_launcher::AppId;
    use sill_services::{Cpx, Permille};
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
        let rows = Rows::new(&DockSettings::default());
        assert_eq!(rows.surface, Cpx::px(160));
        assert_eq!(rows.baseline, Cpx::px(144), "gut + pad above the bottom");
        assert_eq!(rows.pill_top, Cpx::px(88));
        assert_eq!(rows.pill_height, Cpx::px(64));
    }

    #[test]
    fn at_rest_the_pill_is_w0_centred_and_the_hit_reaches_the_edge() {
        // Seven tiles: W0 = 7 x 48 + 6 x 4 + 16 = 376, centred on 1920: 772.
        let f = frame(
            &items(7, false),
            &DockSettings::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        assert_eq!((f.pill.left, f.pill.width), (Cpx::px(772), Cpx::px(376)));
        assert_eq!(f.separator, None);
        assert_eq!(f.hit.top, Cpx::px(88), "the pill's top at rest");
        assert_eq!(f.hit.height, Cpx::px(72), "pill plus gutter");
        assert!(f.tiles.iter().all(|t| t.width == Cpx::px(48)));
        assert_eq!(f.tiles[0].left, Cpx::px(780));
    }

    #[test]
    fn trash_widens_the_pill_by_the_separator_and_stays_centred() {
        let f = frame(
            &items(6, true),
            &DockSettings::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        assert_eq!(f.pill.width, Cpx::px(376 + 13));
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
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        let x = Cpx(rest.tiles[3].left.0 + Cpx::px(24).0);
        let f = frame(
            &items(7, false),
            &DockSettings::default(),
            1920,
            Permille::ONE,
            x,
        );
        assert_eq!(f.tiles[3].width, Cpx::px(96));
        assert_eq!(f.hit.top, Cpx::px(144 - 96));
        assert!(f.tiles[3].contains(x, Cpx::px(100)), "the anchored tile");
    }
}
