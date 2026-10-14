//! The rectangle each icon rests in, for the compositor's minimise targets: the dock's frame at
//! no magnification, so the target holds still while the pointer moves over the dock. Pure.

use sill_model::apps::AppIndex;
use sill_model::desktop::target::{AppRects, Rect};
use sill_model::units::Cpx;
use sill_model::windows::Toplevels;

use crate::dock::frame::Area;
use crate::dock::items::{DockItem, ItemKind, item_windows};

/// `rest`'s tiles (one area per item, in item order) as the rectangles of the apps they stand
/// for. An app goes by its tile's desktop entry and by the app id of each of the tile's windows;
/// when two tiles claim a name the earlier tile keeps it. The Trash has none.
pub fn app_rects(
    items: &[DockItem],
    rest: &[Area],
    toplevels: &Toplevels,
    index: &AppIndex,
) -> AppRects {
    let mut rects = AppRects::default();
    for (item, area) in items.iter().zip(rest) {
        if item.kind == ItemKind::Trash {
            continue;
        }
        let rect = rect_of(area);
        let entry = item.kind.app_id().map(|id| id.0.clone());
        let windows = item_windows(&item.kind, toplevels, index)
            .into_iter()
            .map(|window| window.app_id.clone());
        for name in entry.into_iter().chain(windows) {
            rects.0.entry(name).or_insert(rect);
        }
    }
    rects
}

/// `area` in whole logical pixels.
fn rect_of(area: &Area) -> Rect {
    let whole = |v: Cpx| (v.0 + 50).div_euclid(100);
    let side = |v: Cpx| u32::try_from(whole(v)).unwrap_or(0);
    (
        whole(area.left),
        whole(area.top),
        side(area.width),
        side(area.height),
    )
}

#[cfg(test)]
mod tests {
    use super::{app_rects, rect_of};
    use crate::dock::frame::Area;
    use crate::dock::items::{DockItem, ItemKind};
    use crate::dock::machine::{TileApp, TileId};
    use sill_launcher::AppId;
    use sill_model::apps::AppIndex;
    use sill_model::units::Cpx;
    use sill_model::windows::Toplevels;

    fn area(left: i32, top: i32, side: i32) -> Area {
        Area {
            left: Cpx(left),
            top: Cpx(top),
            width: Cpx(side),
            height: Cpx(side),
        }
    }

    fn item(n: u32, kind: ItemKind) -> DockItem {
        DockItem {
            tile: TileId(n),
            kind,
            app: TileApp::NotRunning,
        }
    }

    #[test]
    fn hundredths_round_to_the_nearest_pixel() {
        assert_eq!(rect_of(&area(1049, 2050, 4800)), (10, 21, 48, 48));
        assert_eq!(rect_of(&area(-151, 0, 4749)), (-2, 0, 47, 47));
    }

    #[test]
    fn each_icon_but_the_trash_is_named_by_its_entry() {
        let items = [
            item(1, ItemKind::Pinned(AppId("term".to_owned()))),
            item(2, ItemKind::Running(AppId("files".to_owned()))),
            item(3, ItemKind::Trash),
        ];
        let rest = [
            area(1000, 500, 4800),
            area(6600, 500, 4800),
            area(9000, 500, 4800),
        ];
        let rects = app_rects(&items, &rest, &Toplevels::default(), &AppIndex::default());
        let want: Vec<_> = rects.0.into_iter().collect();
        assert_eq!(
            want,
            [
                ("files".to_owned(), (66, 5, 48, 48)),
                ("term".to_owned(), (10, 5, 48, 48)),
            ]
        );
    }
}
