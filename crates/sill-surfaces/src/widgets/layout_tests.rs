//! Table tests for `layout.rs`.

use ds::widget::{DesktopGrid, GridCell, LayoutError, Order, WidgetAt, WidgetEdit};
use ds::{WidgetHost, WidgetSize};

use super::{Refused, WidgetKeys, edited, layout_of};
use crate::surfaces::desktop_widgets::{GridSlot, Placement};
use crate::widgets::sizes::Sizes;
use crate::widgets::{WidgetKind, kind_id};

const GRID: DesktopGrid = DesktopGrid {
    columns: 5,
    rows: 3,
};

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

/// The defaults as the desktop layer places them: the month and the batteries at the top right.
fn keys() -> WidgetKeys {
    use WidgetKind::{Battery, Calendar, UpNext};
    WidgetKeys {
        center: vec![Calendar, UpNext, Battery],
        desktop: vec![Calendar, Battery],
        sizes: Sizes::default(),
        placed: vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
    }
}

fn add(kind: WidgetKind, size: WidgetSize, host: WidgetHost) -> WidgetEdit {
    WidgetEdit::Add {
        kind: kind_id(kind),
        size,
        host,
    }
}

#[test]
fn the_keys_read_as_one_layout() {
    let layout = layout_of(&keys());
    let items: Vec<_> = layout
        .items()
        .iter()
        .map(|item| (item.id.0, item.kind.as_str(), item.size, item.at))
        .collect();
    let cell = |column, row| WidgetAt::Desktop(GridCell { column, row });
    assert_eq!(
        items,
        vec![
            (
                1,
                "quire.month",
                WidgetSize::Large,
                WidgetAt::Center(Order(0))
            ),
            (
                2,
                "sill.up-next",
                WidgetSize::Medium,
                WidgetAt::Center(Order(1))
            ),
            (
                3,
                "quire.battery",
                WidgetSize::Medium,
                WidgetAt::Center(Order(2))
            ),
            (4, "quire.month", WidgetSize::Small, cell(4, 0)),
            (5, "quire.battery", WidgetSize::Small, cell(4, 1)),
        ]
    );
    let unplaced = WidgetKeys {
        placed: vec![at(WidgetKind::Calendar, 4, 0)],
        ..keys()
    };
    assert_eq!(
        layout_of(&unplaced).items().len(),
        4,
        "a widget with no cell is left out"
    );
}

#[test]
fn each_edit_writes_the_same_keys_and_places() {
    use WidgetKind::{Battery, Calendar, NowPlaying, UpNext, WorldClock};
    // (name, edit, center, desktop, sizes, places)
    let cases = [
        (
            "add to the desktop: the first free spot scanning columns from the right",
            add(WorldClock, WidgetSize::Medium, WidgetHost::Desktop),
            vec![Calendar, UpNext, Battery],
            vec![Calendar, Battery, WorldClock],
            "",
            vec![at(Calendar, 4, 0), at(Battery, 4, 1), at(WorldClock, 3, 2)],
        ),
        (
            "add to the center at the foot, at a size of its own",
            add(NowPlaying, WidgetSize::Medium, WidgetHost::Tile),
            vec![Calendar, UpNext, Battery, NowPlaying],
            vec![Calendar, Battery],
            "",
            vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
        ),
        (
            "remove from the center",
            WidgetEdit::Remove(ds::catalog::PlacementId(2)),
            vec![Calendar, Battery],
            vec![Calendar, Battery],
            "",
            vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
        ),
        (
            "remove from the desktop",
            WidgetEdit::Remove(ds::catalog::PlacementId(4)),
            vec![Calendar, UpNext, Battery],
            vec![Battery],
            "",
            vec![at(Battery, 4, 1)],
        ),
        (
            "grow the desktop month: its cell cannot hold a large, so the first free spot",
            WidgetEdit::Resize(ds::catalog::PlacementId(4), WidgetSize::Large),
            vec![Calendar, UpNext, Battery],
            vec![Calendar, Battery],
            "desktop.calendar=large",
            vec![at(Calendar, 2, 0), at(Battery, 4, 1)],
        ),
        (
            "a tile's size",
            WidgetEdit::Resize(ds::catalog::PlacementId(2), WidgetSize::Large),
            vec![Calendar, UpNext, Battery],
            vec![Calendar, Battery],
            "center.up_next=large",
            vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
        ),
    ];
    for (name, edit, center, desktop, sizes, placed) in cases {
        let got = edited(&keys(), edit, GRID).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(got.center, center, "{name}");
        assert_eq!(got.desktop, desktop, "{name}");
        assert_eq!(got.sizes.text(), sizes, "{name}");
        assert_eq!(got.placed, placed, "{name}");
    }
}

#[test]
fn a_size_taken_away_goes_back_to_none() {
    let keys = WidgetKeys {
        sizes: Sizes::parse("center.up_next=large"),
        ..keys()
    };
    let got = edited(&keys, WidgetEdit::Remove(ds::catalog::PlacementId(2)), GRID)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got.sizes.text(), "");
}

#[test]
fn a_widget_that_fits_nowhere_stays_in_the_key() {
    let keys = WidgetKeys {
        desktop: vec![
            WidgetKind::Calendar,
            WidgetKind::Battery,
            WidgetKind::UpNext,
        ],
        ..keys()
    };
    let got = edited(&keys, WidgetEdit::Remove(ds::catalog::PlacementId(4)), GRID)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got.desktop, vec![WidgetKind::Battery, WidgetKind::UpNext]);
}

#[test]
fn what_cannot_be_placed_is_refused() {
    // One column of two cells, both taken.
    let full = DesktopGrid {
        columns: 1,
        rows: 2,
    };
    let packed = WidgetKeys {
        placed: vec![
            at(WidgetKind::Calendar, 0, 0),
            at(WidgetKind::Battery, 0, 1),
        ],
        ..keys()
    };
    let foreign = WidgetEdit::Add {
        kind: ds::WidgetKind::fixed("org.example.weather"),
        size: WidgetSize::Small,
        host: WidgetHost::Tile,
    };
    let cases = [
        (
            add(WidgetKind::Battery, WidgetSize::Small, WidgetHost::Desktop),
            keys(),
            GRID,
            Refused::Placed(WidgetKind::Battery),
        ),
        (
            add(WidgetKind::UpNext, WidgetSize::Medium, WidgetHost::Tile),
            keys(),
            GRID,
            Refused::Placed(WidgetKind::UpNext),
        ),
        (
            add(
                WidgetKind::WorldClock,
                WidgetSize::Small,
                WidgetHost::Desktop,
            ),
            packed,
            full,
            Refused::Layout(LayoutError::Full),
        ),
        (
            foreign.clone(),
            keys(),
            GRID,
            Refused::Foreign(ds::WidgetKind::fixed("org.example.weather")),
        ),
    ];
    for (edit, keys, grid, want) in cases {
        assert_eq!(edited(&keys, edit.clone(), grid), Err(want), "{edit:?}");
    }
}
