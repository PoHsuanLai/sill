//! Where a placed widget is drawn on the layer, in logical px: the grid is flush with the
//! layer's right edge (new widgets fill from the top right, `placement::place`), `gap` around it
//! and between its cells. Pure.

use super::placement::{GridSize, GridSlot};
use crate::widgets::{WidgetSize, cells};

/// A widget's box on the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaceBox {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

impl PlaceBox {
    /// The inline declarations that put an element there.
    pub fn style_attr(&self) -> String {
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px;",
            self.left, self.top, self.width, self.height
        )
    }
}

/// The grid's metrics: its cell and gap, and the layer's width it is laid against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridMetrics {
    pub grid: GridSize,
    pub layer_width: u32,
    pub cell: u16,
    pub gap: u16,
}

/// The box of a `size`d widget whose top-left cell is `slot`.
pub fn place_box(slot: GridSlot, size: WidgetSize, metrics: GridMetrics) -> PlaceBox {
    let cell = u32::from(metrics.cell);
    let gap = u32::from(metrics.gap);
    let step = cell + gap;
    let span = |count: u16| u32::from(count) * cell + u32::from(count.saturating_sub(1)) * gap;
    let used = u32::from(metrics.grid.columns) * step + gap;
    let origin = metrics.layer_width.saturating_sub(used);
    let (columns, rows) = cells(size);
    PlaceBox {
        left: origin + gap + u32::from(slot.column) * step,
        top: gap + u32::from(slot.row) * step,
        width: span(columns),
        height: span(rows),
    }
}

#[cfg(test)]
mod tests {
    use super::{GridMetrics, PlaceBox, place_box};
    use crate::surfaces::desktop_widgets::placement::{GridSize, GridSlot};
    use crate::widgets::WidgetSize;

    #[test]
    fn boxes_sit_on_a_grid_flush_with_the_right_edge() {
        // 1000 px wide: five 164 px cells and six 16 px gaps take 916, so the grid starts at 84.
        let metrics = GridMetrics {
            grid: GridSize {
                columns: 5,
                rows: 3,
            },
            layer_width: 1000,
            cell: 164,
            gap: 16,
        };
        let slot = |column, row| GridSlot { column, row };
        let at = |left, top, width, height| PlaceBox {
            left,
            top,
            width,
            height,
        };
        let cases = [
            (slot(4, 0), WidgetSize::Small, at(820, 16, 164, 164)),
            (slot(0, 0), WidgetSize::Small, at(100, 16, 164, 164)),
            (slot(3, 1), WidgetSize::Medium, at(640, 196, 344, 164)),
            (slot(1, 1), WidgetSize::Large, at(280, 196, 344, 344)),
        ];
        for (slot, size, want) in cases {
            assert_eq!(place_box(slot, size, metrics), want, "{slot:?} {size:?}");
        }
        assert_eq!(
            at(1, 2, 3, 4).style_attr(),
            "left:1px;top:2px;width:3px;height:4px;"
        );
    }
}
