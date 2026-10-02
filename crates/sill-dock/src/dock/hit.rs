//! What the pointer is over, from the frame on screen (design/10 §10.3.3: x picks the tile, y
//! only decides inside or out), and where a dragged tile would land (§10.3.8).

use sill_model::units::Cpx;

use crate::dock::frame::Frame;
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{DropZone, Hit, Point, TileApp, TileId};

/// The tile under `at`, the pill, or outside the hit element.
pub fn hit(frame: &Frame, items: &[DockItem], at: Point) -> Hit {
    if !frame.hit.contains(at.x, at.y) {
        return Hit::Outside;
    }
    let gap = half_gap(frame);
    let under = frame
        .tiles
        .iter()
        .zip(items)
        .find(|(area, _)| at.x >= Cpx(area.left.0 - gap) && at.x < Cpx(area.right().0 + gap));
    match under {
        Some((_, item)) => Hit::Tile {
            tile: item.tile,
            app: hit_app(item),
        },
        None => Hit::Pill,
    }
}

/// The app state a click decides on. The Trash tile never launches an app the dock could see
/// map a window, so it reads as a running app and a click activates it (the component opens
/// `trash:///`), which spares it a ten-second launch bounce.
fn hit_app(item: &DockItem) -> TileApp {
    match item.kind {
        ItemKind::Trash => TileApp::Inactive,
        ItemKind::Pinned(_) | ItemKind::Running(_) | ItemKind::Unmatched(_) => item.app,
    }
}

/// Half the gap between neighbouring tiles: the slots tile the pill without holes.
fn half_gap(frame: &Frame) -> i32 {
    match frame.tiles.as_slice() {
        [first, second, ..] => (second.left.0 - first.right().0).max(0) / 2,
        _ => 0,
    }
}

/// Where `dragged` would land with the pointer at `at`: beyond the pill grown by `remove`,
/// removal; else the index among the other app tiles whose rest centres lie left of the
/// pointer (the index a `dock.json` move takes, after the tile left its place).
pub fn drop_zone(
    frame: &Frame,
    items: &[DockItem],
    dragged: TileId,
    at: Point,
    remove: Cpx,
) -> DropZone {
    if !frame.pill.inflate(remove).contains(at.x, at.y) {
        return DropZone::Remove;
    }
    let before = frame
        .centers
        .iter()
        .zip(items)
        .filter(|(_, item)| item.tile != dragged && item.kind != ItemKind::Trash)
        .filter(|(center, _)| **center < at.x)
        .count();
    DropZone::Slot(u32::try_from(before).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::{drop_zone, hit};
    use crate::dock::frame::frame;
    use crate::dock::items::{DockItem, ItemKind};
    use crate::dock::machine::{DropZone, Hit, Point, TileApp, TileId};
    use sill_launcher::AppId;
    use sill_model::units::{Cpx, Permille};
    use sill_settings::DockSettings;

    fn items() -> Vec<DockItem> {
        let mut items: Vec<DockItem> = (0..3)
            .map(|i| DockItem {
                tile: TileId(i),
                kind: ItemKind::Pinned(AppId(format!("app{i}"))),
                app: TileApp::NotRunning,
            })
            .collect();
        items.push(DockItem {
            tile: TileId(9),
            kind: ItemKind::Trash,
            app: TileApp::NotRunning,
        });
        items
    }

    fn at(x: i32, y: i32) -> Point {
        Point {
            x: Cpx::px(x),
            y: Cpx::px(y),
        }
    }

    #[test]
    fn x_picks_the_tile_and_the_gutter_still_hits() {
        let items = items();
        let f = frame(
            &items,
            &DockSettings::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        let first = f.tiles[0].left.0 / 100;
        let trash = f.tiles[3].left.0 / 100;
        let cases = [
            ("the first tile", at(first + 10, 96), Some(TileId(0))),
            ("the gutter under it", at(first + 10, 133), Some(TileId(0))),
            ("the gap right of it", at(first + 49, 96), Some(TileId(0))),
            ("above the pill", at(first + 10, 56), None),
            ("the Trash tile", at(trash + 5, 96), Some(TileId(9))),
        ];
        for (name, point, want) in cases {
            assert_eq!(hit(&f, &items, point).tile(), want, "{name}");
        }
        assert_eq!(hit(&f, &items, at(first + 10, 56)), Hit::Outside);
        assert_eq!(
            hit(&f, &items, at(trash + 5, 96)),
            Hit::Tile {
                tile: TileId(9),
                app: TileApp::Inactive
            },
            "Trash activates, never launch-bounces"
        );
    }

    #[test]
    fn a_drag_lands_between_rest_centres_or_removes_past_the_threshold() {
        let items = items();
        let f = frame(
            &items,
            &DockSettings::default(),
            1920,
            Permille::ZERO,
            Cpx(0),
        );
        let c = |i: usize| f.centers[i].0 / 100;
        let remove = Cpx::px(64);
        let cases = [
            ("left of everything", at(c(0) - 10, 120), DropZone::Slot(0)),
            ("between 1 and 2", at(c(1) + 5, 120), DropZone::Slot(1)),
            ("past the last app", at(c(2) + 5, 120), DropZone::Slot(2)),
            (
                "100 px above the pill",
                at(c(1), 90 - 100),
                DropZone::Remove,
            ),
            (
                "60 px above the pill",
                at(c(1) + 5, 90 - 60),
                DropZone::Slot(1),
            ),
        ];
        for (name, point, want) in cases {
            assert_eq!(
                drop_zone(&f, &items, TileId(0), point, remove),
                want,
                "{name}"
            );
        }
    }
}
