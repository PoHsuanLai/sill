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
    let fit = |length: u32| {
        let step = u32::from(cell) + u32::from(gap);
        let count = length.saturating_sub(u32::from(gap)).checked_div(step);
        u16::try_from(count.unwrap_or(0)).unwrap_or(u16::MAX)
    };
    GridSize {
        columns: fit(width),
        rows: fit(height),
    }
}

/// A place for each of `kinds` (each `size`d by `size_of`): a saved placement is kept when it
/// still fits and overlaps nothing kept before it; every other widget takes the first free
/// spot scanning columns from the right, top to bottom. A widget that fits nowhere is left out.
pub fn place(
    kinds: &[(WidgetKind, WidgetSize)],
    saved: &[Placement],
    grid: GridSize,
) -> Vec<Placement> {
    let mut taken: Vec<Footprint> = Vec::new();
    let kept: Vec<Option<GridSlot>> = kinds
        .iter()
        .map(|&(kind, size)| {
            let slot = saved.iter().find(|p| p.kind == kind)?.slot;
            let spot = Footprint::new(slot, size);
            spot.free_in(grid, &taken).then(|| {
                taken.push(spot);
                slot
            })
        })
        .collect();
    kinds
        .iter()
        .zip(kept)
        .filter_map(|(&(kind, size), kept)| {
            let slot = kept.or_else(|| {
                let spot = first_free(size, grid, &taken)?;
                taken.push(spot);
                Some(spot.slot)
            })?;
            Some(Placement { kind, slot })
        })
        .collect()
}

/// The cells one widget covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footprint {
    slot: GridSlot,
    columns: u16,
    rows: u16,
}

impl Footprint {
    fn new(slot: GridSlot, size: WidgetSize) -> Footprint {
        let (columns, rows) = size.cells();
        Footprint {
            slot,
            columns,
            rows,
        }
    }

    /// Inside `grid` and clear of every one of `taken`.
    fn free_in(&self, grid: GridSize, taken: &[Footprint]) -> bool {
        let inside = u32::from(self.slot.column) + u32::from(self.columns)
            <= u32::from(grid.columns)
            && u32::from(self.slot.row) + u32::from(self.rows) <= u32::from(grid.rows);
        inside && taken.iter().all(|other| !self.overlaps(other))
    }

    fn overlaps(&self, other: &Footprint) -> bool {
        let apart = |a: u16, a_len: u16, b: u16, b_len: u16| {
            u32::from(a) + u32::from(a_len) <= u32::from(b)
                || u32::from(b) + u32::from(b_len) <= u32::from(a)
        };
        !(apart(
            self.slot.column,
            self.columns,
            other.slot.column,
            other.columns,
        ) || apart(self.slot.row, self.rows, other.slot.row, other.rows))
    }
}

/// The first spot `size` takes free of `taken`: columns from the right, each top to bottom.
fn first_free(size: WidgetSize, grid: GridSize, taken: &[Footprint]) -> Option<Footprint> {
    (0..grid.columns)
        .rev()
        .flat_map(|column| (0..grid.rows).map(move |row| GridSlot { column, row }))
        .map(|slot| Footprint::new(slot, size))
        .find(|spot| spot.free_in(grid, taken))
}

#[cfg(test)]
mod tests {
    use sill_settings::WidgetKind;

    use super::{GridSize, GridSlot, Placement, grid_size, place};
    use crate::widgets::WidgetSize;

    #[test]
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

    #[test]
    fn an_overlapping_save_moves_and_what_fits_nowhere_is_left_out() {
        let grid = GridSize {
            columns: 2,
            rows: 2,
        };
        let at = |kind, column, row| Placement {
            kind,
            slot: GridSlot { column, row },
        };
        // UpNext's saved place overlaps the Calendar's, kept first: it moves to the free row.
        let kinds = [
            (WidgetKind::Calendar, WidgetSize::Small),
            (WidgetKind::UpNext, WidgetSize::Medium),
            (WidgetKind::NowPlaying, WidgetSize::Large),
            (WidgetKind::Battery, WidgetSize::Small),
        ];
        let saved = [at(WidgetKind::Calendar, 1, 0), at(WidgetKind::UpNext, 0, 0)];
        assert_eq!(
            place(&kinds, &saved, grid),
            vec![
                at(WidgetKind::Calendar, 1, 0),
                at(WidgetKind::UpNext, 0, 1),
                at(WidgetKind::Battery, 0, 0),
            ]
        );
        let empty = GridSize {
            columns: 0,
            rows: 0,
        };
        assert_eq!(place(&kinds, &saved, empty), Vec::new());
    }
}
