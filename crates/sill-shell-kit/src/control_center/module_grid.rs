//! ModuleGrid: the control center's grid of modules (design/13 section 13.3.7). Its columns, gap
//! and padding are props, so the columns, `grid_gap_px` and `grid_padding_px`
//! (design/22-SETTINGS.md section 3.13) reach the grid and not only the popup's size estimate
//!. A `ModuleTile { span: TileSpan::Full }` or a `ModulePanel` spans every
//! column.

use crate::tokens::control_center::CONTROL_CENTER;
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::root::common::Common;

/// How many equal columns the grid has; at least one (a zero is read as one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridColumns(pub u16);

impl Default for GridColumns {
    /// Two columns, as Control Centre's tiles.
    fn default() -> Self {
        GridColumns(2)
    }
}

/// The grid's geometry, written inline as the three custom properties `module_grid.css` reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridMetrics {
    /// The number of columns.
    pub columns: GridColumns,
    /// Between columns and rows.
    pub gap: Px,
    /// Inside the grid's edge.
    pub padding: Px,
}

impl GridMetrics {
    /// `--grid-columns:2;--grid-gap:8px;--grid-padding:12px`: lengths rounded to whole pixels
    /// and held at zero or more, as the keys are `Px` (u16).
    pub fn style_attr(&self) -> String {
        let whole = |px: Px| px.0.round().max(0.0) as u32;
        format!(
            "--grid-columns:{};--grid-gap:{}px;--grid-padding:{}px",
            self.columns.0.max(1),
            whole(self.gap),
            whole(self.padding)
        )
    }
}

/// `children` (`ModuleTile`s and `ModulePanel`s) in `columns` equal columns, `gap` apart, with
/// `padding` inside the grid. The defaults are the control center's sizes (2, 10, 10;
/// `CONTROL_CENTER`, design/29-SIZING.md).
#[component]
pub fn ModuleGrid(
    #[props(default)] columns: GridColumns,
    #[props(default = Px(f32::from(CONTROL_CENTER.gap.0)))] gap: Px,
    #[props(default = Px(f32::from(CONTROL_CENTER.padding.0)))] padding: Px,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    let style = GridMetrics {
        columns,
        gap,
        padding,
    }
    .style_attr();
    let class = common.class("ds-module-grid");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            style,
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GridColumns, GridMetrics};
    use ds::base::geometry::units::Px;

    #[test]
    fn the_metrics_are_written_whole_and_never_below_one_column() {
        let keys = GridMetrics {
            columns: GridColumns::default(),
            gap: Px(8.0),
            padding: Px(12.0),
        };
        assert_eq!(
            keys.style_attr(),
            "--grid-columns:2;--grid-gap:8px;--grid-padding:12px"
        );
        let odd = GridMetrics {
            columns: GridColumns(0),
            gap: Px(7.6),
            padding: Px(-3.0),
        };
        assert_eq!(
            odd.style_attr(),
            "--grid-columns:1;--grid-gap:8px;--grid-padding:0px"
        );
    }
}
