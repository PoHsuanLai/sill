//! Where the person put their widgets, as data (design/23-WIDGETS.md section 9.7): the catalog's
//! placements with a widget's kind, size and position, the position a grid cell on the desktop
//! or an order in the notification center's column. A host keeps a [`WidgetLayout`] in its
//! settings; the widget gallery and the desktop's drag hand it [`WidgetEdit`]s, and [`apply`]
//! turns an edit into the next layout or says why it cannot (pure: no clock, no I/O).

use crate::catalog::placement::{Placed, PlacementId, Placements};
use crate::widget::contract::WidgetKind;
use crate::widget::kind::{WidgetHost, WidgetSize};
use serde::{Deserialize, Serialize};

/// A cell of the desktop's widget grid, from its top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GridCell {
    /// Columns from the left.
    pub column: u16,
    /// Rows from the top.
    pub row: u16,
}

/// A place in the notification center's column, from the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Order(pub u16);

/// Where a widget is: the surface and the position on it, together, so a desktop widget always
/// has a cell and a center widget an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetAt {
    /// On the desktop, its top left at this cell.
    Desktop(GridCell),
    /// In the notification center, at this place in the column.
    Center(Order),
}

impl WidgetAt {
    /// The host this position is on.
    pub fn host(self) -> WidgetHost {
        match self {
            WidgetAt::Desktop(_) => WidgetHost::Desktop,
            WidgetAt::Center(_) => WidgetHost::Tile,
        }
    }
}

/// One placed widget.
pub type WidgetPlacement = Placed<WidgetKind, WidgetSize, WidgetAt>;

/// Every placed widget, on both surfaces.
pub type WidgetLayout = Placements<WidgetKind, WidgetSize, WidgetAt>;

/// The desktop's widget grid, in cells (the host measures it from the output and the grid unit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DesktopGrid {
    /// Columns.
    pub columns: u16,
    /// Rows.
    pub rows: u16,
}

/// A change the person makes to the layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetEdit {
    /// Add `kind` at `size` to `host`: the desktop's first free cell, or the column's foot.
    Add {
        /// What.
        kind: WidgetKind,
        /// How big.
        size: WidgetSize,
        /// Which surface.
        host: WidgetHost,
    },
    /// Take a widget away.
    Remove(PlacementId),
    /// Change a widget's size, keeping its cell when the new footprint fits there.
    Resize(PlacementId, WidgetSize),
    /// Put a widget somewhere else (a drop on the desktop, a reorder in the column).
    Move(PlacementId, WidgetAt),
}

/// Why an edit cannot be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum LayoutError {
    /// No free cell on the desktop fits the widget.
    #[error("no free cell on the desktop fits the widget")]
    Full,
    /// The cell asked for is off the grid or covered by another widget.
    #[error("the cell is off the grid or taken")]
    Taken,
    /// The layout holds no such widget.
    #[error("the layout holds no widget {0:?}")]
    Unknown(PlacementId),
}

/// The cells a `size` widget covers: (columns, rows).
pub fn cells(size: WidgetSize) -> (u16, u16) {
    match size {
        WidgetSize::Small => (1, 1),
        WidgetSize::Medium => (2, 1),
        WidgetSize::Large => (2, 2),
    }
}

/// The cells one widget covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footprint {
    cell: GridCell,
    size: WidgetSize,
}

impl Footprint {
    fn inside(self, grid: DesktopGrid) -> Fits {
        let (columns, rows) = cells(self.size);
        let right = u32::from(self.cell.column) + u32::from(columns);
        let bottom = u32::from(self.cell.row) + u32::from(rows);
        Fits::of(right <= u32::from(grid.columns) && bottom <= u32::from(grid.rows))
    }

    fn overlaps(self, other: Footprint) -> Fits {
        let span = |at: u16, len: u16| (u32::from(at), u32::from(at) + u32::from(len));
        let apart = |(a0, a1): (u32, u32), (b0, b1): (u32, u32)| a1 <= b0 || b1 <= a0;
        let (sc, sr) = cells(self.size);
        let (oc, or) = cells(other.size);
        let clear = apart(span(self.cell.column, sc), span(other.cell.column, oc))
            || apart(span(self.cell.row, sr), span(other.cell.row, or));
        Fits::of(!clear)
    }
}

/// A yes or no about fitting, so no `bool` crosses a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fits {
    Yes,
    No,
}

impl Fits {
    fn of(yes: bool) -> Fits {
        if yes { Fits::Yes } else { Fits::No }
    }
}

/// The desktop footprints in `layout`, less `except`.
fn taken(layout: &WidgetLayout, except: Option<PlacementId>) -> Vec<Footprint> {
    layout
        .items()
        .iter()
        .filter(|item| Some(item.id) != except)
        .filter_map(|item| match item.at {
            WidgetAt::Desktop(cell) => Some(Footprint {
                cell,
                size: item.size,
            }),
            WidgetAt::Center(_) => None,
        })
        .collect()
}

/// Whether `spot` lies on the grid, clear of `taken`.
fn free(spot: Footprint, grid: DesktopGrid, taken: &[Footprint]) -> Fits {
    let clear = taken.iter().all(|&other| spot.overlaps(other) == Fits::No);
    Fits::of(spot.inside(grid) == Fits::Yes && clear)
}

/// The first free cell for a `size` widget, scanning the columns from the right (the desktop's
/// widgets gather at its top right, as sill places them), each from the top.
pub fn first_free(
    layout: &WidgetLayout,
    size: WidgetSize,
    grid: DesktopGrid,
    except: Option<PlacementId>,
) -> Option<GridCell> {
    let taken = taken(layout, except);
    (0..grid.columns)
        .rev()
        .flat_map(|column| (0..grid.rows).map(move |row| GridCell { column, row }))
        .find(|&cell| free(Footprint { cell, size }, grid, &taken) == Fits::Yes)
}

/// The column's next order: one past the last.
fn next_order(layout: &WidgetLayout) -> Order {
    let last = layout.items().iter().filter_map(|item| match item.at {
        WidgetAt::Center(Order(order)) => Some(order),
        WidgetAt::Desktop(_) => None,
    });
    Order(last.max().map_or(0, |order| order.saturating_add(1)))
}

/// `layout` after `edit` on a desktop of `grid`.
pub fn apply(
    layout: WidgetLayout,
    edit: WidgetEdit,
    grid: DesktopGrid,
) -> Result<WidgetLayout, LayoutError> {
    match edit {
        WidgetEdit::Add { kind, size, host } => {
            let at = match host {
                WidgetHost::Desktop => WidgetAt::Desktop(
                    first_free(&layout, size, grid, None).ok_or(LayoutError::Full)?,
                ),
                WidgetHost::Tile => WidgetAt::Center(next_order(&layout)),
            };
            Ok(layout.added(kind, size, at).0)
        }
        WidgetEdit::Remove(id) => {
            layout.get(id).ok_or(LayoutError::Unknown(id))?;
            Ok(layout.removed(id))
        }
        WidgetEdit::Resize(id, size) => {
            let item = layout.get(id).ok_or(LayoutError::Unknown(id))?;
            let at = match item.at {
                WidgetAt::Desktop(cell) => {
                    let others = taken(&layout, Some(id));
                    match free(Footprint { cell, size }, grid, &others) {
                        Fits::Yes => WidgetAt::Desktop(cell),
                        Fits::No => WidgetAt::Desktop(
                            first_free(&layout, size, grid, Some(id)).ok_or(LayoutError::Full)?,
                        ),
                    }
                }
                center @ WidgetAt::Center(_) => center,
            };
            Ok(layout.changed(id, |item| Placed { size, at, ..item }))
        }
        WidgetEdit::Move(id, at) => {
            let item = layout.get(id).ok_or(LayoutError::Unknown(id))?;
            if let WidgetAt::Desktop(cell) = at {
                let spot = Footprint {
                    cell,
                    size: item.size,
                };
                if free(spot, grid, &taken(&layout, Some(id))) == Fits::No {
                    return Err(LayoutError::Taken);
                }
            }
            Ok(layout.changed(id, |item| Placed { at, ..item }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DesktopGrid, GridCell, LayoutError, Order, WidgetAt, WidgetEdit, WidgetLayout, apply,
    };
    use crate::catalog::placement::PlacementId;
    use crate::widget::contract::WidgetKind;
    use crate::widget::kind::{WidgetHost, WidgetSize};

    const GRID: DesktopGrid = DesktopGrid {
        columns: 4,
        rows: 3,
    };

    fn add(
        layout: WidgetLayout,
        name: &'static str,
        size: WidgetSize,
        host: WidgetHost,
    ) -> WidgetLayout {
        let edit = WidgetEdit::Add {
            kind: WidgetKind::fixed(name),
            size,
            host,
        };
        apply(layout, edit, GRID).expect("fits")
    }

    fn at(layout: &WidgetLayout, id: u32) -> Option<WidgetAt> {
        layout.get(PlacementId(id)).map(|item| item.at)
    }

    fn cell(column: u16, row: u16) -> WidgetAt {
        WidgetAt::Desktop(GridCell { column, row })
    }

    #[test]
    fn widgets_fill_the_desktop_from_its_top_right() {
        let layout = add(
            WidgetLayout::default(),
            "quire.battery",
            WidgetSize::Small,
            WidgetHost::Desktop,
        );
        let layout = add(
            layout,
            "quire.month",
            WidgetSize::Small,
            WidgetHost::Desktop,
        );
        let layout = add(
            layout,
            "quire.world-clock",
            WidgetSize::Medium,
            WidgetHost::Desktop,
        );
        assert_eq!(at(&layout, 1), Some(cell(3, 0)));
        assert_eq!(at(&layout, 2), Some(cell(3, 1)));
        assert_eq!(
            at(&layout, 3),
            Some(cell(2, 2)),
            "a medium needs two free columns"
        );
        let layout = add(layout, "quire.battery", WidgetSize::Small, WidgetHost::Tile);
        let layout = add(layout, "quire.month", WidgetSize::Large, WidgetHost::Tile);
        assert_eq!(at(&layout, 4), Some(WidgetAt::Center(Order(0))));
        assert_eq!(at(&layout, 5), Some(WidgetAt::Center(Order(1))));
    }

    #[test]
    fn a_resize_keeps_the_cell_when_it_fits_and_moves_when_not() {
        let layout = add(
            WidgetLayout::default(),
            "a",
            WidgetSize::Small,
            WidgetHost::Desktop,
        );
        let layout = add(layout, "b", WidgetSize::Small, WidgetHost::Desktop);
        let layout =
            apply(layout, WidgetEdit::Move(PlacementId(1), cell(0, 0)), GRID).expect("free");
        let grown = apply(
            layout.clone(),
            WidgetEdit::Resize(PlacementId(1), WidgetSize::Large),
            GRID,
        )
        .expect("fits");
        assert_eq!(at(&grown, 1), Some(cell(0, 0)));
        let layout =
            apply(layout, WidgetEdit::Move(PlacementId(1), cell(2, 1)), GRID).expect("free");
        let grown = apply(
            layout,
            WidgetEdit::Resize(PlacementId(1), WidgetSize::Medium),
            GRID,
        )
        .expect("fits elsewhere");
        assert_eq!(
            at(&grown, 1),
            Some(cell(2, 0)),
            "b holds (3, 1), so the medium moves to the first free spot"
        );
    }

    #[test]
    fn a_move_onto_another_widget_or_off_the_grid_is_refused() {
        let layout = add(
            WidgetLayout::default(),
            "a",
            WidgetSize::Medium,
            WidgetHost::Desktop,
        );
        let layout = add(layout, "b", WidgetSize::Small, WidgetHost::Desktop);
        assert_eq!(
            apply(
                layout.clone(),
                WidgetEdit::Move(PlacementId(2), cell(2, 0)),
                GRID
            ),
            Err(LayoutError::Taken)
        );
        assert_eq!(
            apply(
                layout.clone(),
                WidgetEdit::Move(PlacementId(1), cell(3, 2)),
                GRID
            ),
            Err(LayoutError::Taken),
            "a medium at the last column hangs off the grid"
        );
        assert_eq!(
            apply(layout.clone(), WidgetEdit::Remove(PlacementId(9)), GRID),
            Err(LayoutError::Unknown(PlacementId(9)))
        );
        let removed = apply(layout, WidgetEdit::Remove(PlacementId(1)), GRID).expect("held");
        assert_eq!(removed.items().len(), 1);
    }

    #[test]
    fn a_full_desktop_says_so() {
        let tiny = DesktopGrid {
            columns: 1,
            rows: 1,
        };
        let edit = |size| WidgetEdit::Add {
            kind: WidgetKind::fixed("a"),
            size,
            host: WidgetHost::Desktop,
        };
        let layout =
            apply(WidgetLayout::default(), edit(WidgetSize::Small), tiny).expect("one fits");
        assert_eq!(
            apply(layout, edit(WidgetSize::Small), tiny),
            Err(LayoutError::Full)
        );
    }

    #[test]
    fn the_layout_is_plain_data_in_settings() {
        let layout = add(
            WidgetLayout::default(),
            "quire.battery",
            WidgetSize::Small,
            WidgetHost::Desktop,
        );
        let layout = add(layout, "quire.month", WidgetSize::Large, WidgetHost::Tile);
        let json = serde_json::to_string(&layout).expect("serialises");
        assert_eq!(
            json,
            r#"{"items":[{"id":1,"kind":"quire.battery","size":"small","at":{"desktop":{"column":3,"row":0}}},{"id":2,"kind":"quire.month","size":"large","at":{"center":0}}],"next":3}"#
        );
        let back: WidgetLayout = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, layout);
    }
}
