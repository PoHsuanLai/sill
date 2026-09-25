//! Where each desktop widget sits: a grid of `widgets.desktop_cell_px` cells with
//! `widgets.desktop_gap_px` between them and around them, filled from the top right (macOS
//! places new desktop widgets at the top left; sill's right side is where the notification
//! center opens, and its left side is where windows open, so the choice is lane b's to confirm
//! with the user). Pure.

use sill_settings::WidgetKind;

use crate::widgets::WidgetSize;

/// A cell: column 0 is the leftmost, row 0 the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridSlot {
    pub column: u16,
    pub row: u16,
}

/// How many cells fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridSize {
    pub columns: u16,
    pub rows: u16,
}

/// One widget's place: its top-left cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placement {
    pub kind: WidgetKind,
    pub slot: GridSlot,
}

/// The grid that fits `width` x `height` logical px (the layer's size, inside the reserves):
/// `n` cells take `n * cell + (n + 1) * gap`.
pub fn grid_size(width: u32, height: u32, cell: u16, gap: u16) -> GridSize {
    let _ = (width, height, cell, gap);
    todo!("M10 fill: b-widgets")
}

/// A place for each of `kinds` (each `size`d by `size_of`): a saved placement is kept when it
/// still fits and overlaps nothing kept before it; every other widget takes the first free
/// spot scanning columns from the right, top to bottom. A widget that fits nowhere is left out.
pub fn place(
    kinds: &[(WidgetKind, WidgetSize)],
    saved: &[Placement],
    grid: GridSize,
) -> Vec<Placement> {
    let _ = (kinds, saved, grid);
    todo!("M10 fill: b-widgets")
}

#[cfg(test)]
mod tests {
    use sill_settings::WidgetKind;

    use super::{GridSize, GridSlot, Placement, grid_size, place};
    use crate::widgets::WidgetSize;

    #[test]
    #[ignore = "M10 fill: b-widgets"]
    fn the_grid_counts_cells_and_gaps() {
        assert_eq!(
            grid_size(1920, 1000, 164, 16),
            GridSize {
                columns: 10,
                rows: 5
            }
        );
        assert_eq!(
            grid_size(100, 100, 164, 16),
            GridSize {
                columns: 0,
                rows: 0
            }
        );
    }

    #[test]
    #[ignore = "M10 fill: b-widgets"]
    fn saved_places_are_kept_and_new_widgets_fill_from_the_top_right() {
        let grid = GridSize {
            columns: 4,
            rows: 3,
        };
        let at = |kind, column, row| Placement {
            kind,
            slot: GridSlot { column, row },
        };
        let kinds = [
            (WidgetKind::Calendar, WidgetSize::Small),
            (WidgetKind::UpNext, WidgetSize::Medium),
            (WidgetKind::Battery, WidgetSize::Small),
        ];
        let saved = [at(WidgetKind::Calendar, 0, 0)];
        assert_eq!(
            place(&kinds, &saved, grid),
            vec![
                at(WidgetKind::Calendar, 0, 0),
                at(WidgetKind::UpNext, 2, 0),
                at(WidgetKind::Battery, 3, 1),
            ]
        );
        let stale = [at(WidgetKind::Calendar, 9, 9)];
        assert_eq!(
            place(&kinds[..1], &stale, grid),
            vec![at(WidgetKind::Calendar, 3, 0)]
        );
    }
}
