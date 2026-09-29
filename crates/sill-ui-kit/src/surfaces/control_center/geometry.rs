//! The control center's grid and size (design/13 §13.3.7, design/29: width 320, two columns,
//! gap 10, padding 10, sliders spanning both columns; height fits content, at most
//! `output_h - bar_h - 16`, then scrolls), pure. Every number is a `control_center.*` or
//! `bar.*` key; the modules' own heights are quire's sizes (`sizes.rs`).
//!
//! The popup's size is estimated, not measured: shell-host's `PopupSize::FitContent` never
//! measures the document, so the bar sizes its popup from [`estimate_size`] and repositions
//! it on a page change, as it does for its menus (`bar/geometry.rs`).

use shell_host::Size;
use sill_settings::{BarSettings, ControlCenterModule, ControlCenterSettings};

use super::modules::{Span, module};
use super::page::CcPage;
use super::sizes::{detail_height, module_height};

/// One module's cell in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placed {
    pub module: ControlCenterModule,
    /// 0-based.
    pub row: u16,
    /// 0-based; a `Row` module is always in column 0.
    pub column: u16,
    pub span: Span,
}

/// Where the next module goes while [`place`] walks the list.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    row: u16,
    /// The next free column of `row`; 0 on a fresh row.
    column: u16,
}

/// `modules` laid out in order, `columns` wide: a `One` module takes the next free column of
/// the current row; a `Row` module starts a new row (leaving any free column of the current row
/// empty) and fills it.
pub fn place(modules: &[ControlCenterModule], columns: u16) -> Vec<Placed> {
    let columns = columns.max(1);
    let start = Cursor { row: 0, column: 0 };
    modules
        .iter()
        .scan(start, |at, &kind| {
            let span = module(kind).span();
            let fresh = match (span, at.column) {
                (_, 0) => *at,
                (Span::One, column) if column < columns => *at,
                _ => Cursor {
                    row: at.row + 1,
                    column: 0,
                },
            };
            let placed = Placed {
                module: kind,
                row: fresh.row,
                column: fresh.column,
                span,
            };
            *at = match span {
                Span::One if fresh.column + 1 < columns => Cursor {
                    row: fresh.row,
                    column: fresh.column + 1,
                },
                Span::One | Span::Row => Cursor {
                    row: fresh.row + 1,
                    column: 0,
                },
            };
            Some(placed)
        })
        .collect()
}

/// The panel's greatest height on an output `output_height` tall.
pub fn height_cap(output_height: i32, bar: &BarSettings, cc: &ControlCenterSettings) -> i32 {
    (output_height - i32::from(bar.height_px.0) - i32::from(cc.bottom_margin_px.0)).max(0)
}

/// The panel's estimated size (before the popup's shadow room), at most `cap` tall: `width_px`
/// wide; on the grid, its padding plus `placed`'s rows (each module's own height) and gaps; on a
/// detail page, its header plus `list_rows` rows.
pub fn estimate_size(
    cc: &ControlCenterSettings,
    page: CcPage,
    placed: &[Placed],
    list_rows: usize,
    cap: i32,
) -> Size {
    let pad = i32::from(cc.grid_padding_px.0);
    let gap = i32::from(cc.grid_gap_px.0);
    let inner = match page {
        CcPage::Modules => grid_height(placed, gap),
        CcPage::Detail(_) => detail_height(list_rows),
    };
    Size {
        width: i32::from(cc.width_px.0),
        height: (inner + 2 * pad).min(cap),
    }
}

/// The grid's rows, each as tall as its tallest module, and the gaps between them.
fn grid_height(placed: &[Placed], gap: i32) -> i32 {
    let rows = placed.iter().fold(Vec::<i32>::new(), |mut rows, cell| {
        let row = usize::from(cell.row);
        if rows.len() <= row {
            rows.resize(row + 1, 0);
        }
        rows[row] = rows[row].max(module_height(cell.module));
        rows
    });
    let count = i32::try_from(rows.len()).unwrap_or(0);
    rows.iter().sum::<i32>() + gap * (count - 1).max(0)
}
#[cfg(test)]
mod tests {
    use super::{Placed, Span, height_cap, place};
    use sill_settings::{BarSettings, ControlCenterModule, ControlCenterSettings};

    #[test]
    fn the_default_modules_lay_out_in_two_columns() {
        use ControlCenterModule::{
            Appearance, Battery, Bluetooth, Display, Focus, KeyboardBrightness, NowPlaying, Sound,
            Wifi,
        };
        let cells: Vec<(ControlCenterModule, u16, u16, Span)> =
            place(&ControlCenterSettings::default().modules, 2)
                .into_iter()
                .map(|p: Placed| (p.module, p.row, p.column, p.span))
                .collect();
        assert_eq!(
            cells,
            vec![
                (Wifi, 0, 0, Span::One),
                (Bluetooth, 0, 1, Span::One),
                (Focus, 1, 0, Span::One),
                (Display, 2, 0, Span::Row),
                (KeyboardBrightness, 3, 0, Span::Row),
                (Sound, 4, 0, Span::Row),
                (NowPlaying, 5, 0, Span::Row),
                (Appearance, 6, 0, Span::Row),
                (Battery, 7, 0, Span::Row),
            ]
        );
    }

    #[test]
    fn the_panel_stops_short_of_the_bottom_edge() {
        let (bar, cc) = (BarSettings::default(), ControlCenterSettings::default());
        // 1080 - bar 24 - margin 16.
        assert_eq!(height_cap(1080, &bar, &cc), 1040);
    }

    #[test]
    fn the_estimate_follows_the_page_and_stops_at_the_cap() {
        use crate::surfaces::control_center::{CcPage, Detail, estimate_size};
        use sill_settings::ControlCenterModule::{Bluetooth, Focus, Sound, Wifi};
        let cc = ControlCenterSettings::default();
        let placed = place(&[Wifi, Bluetooth, Focus, Sound], 2);
        // (page, list rows, cap, width, height): padding 10 twice; tiles 56, gap 10, level 64;
        // a pane's head 36, gap 6, rows 44 (or the 41 empty line), gap 6, foot 29.
        let cases = [
            (CcPage::Modules, 0, 1032, 320, 20 + 56 + 10 + 56 + 10 + 64),
            (CcPage::Modules, 0, 200, 320, 200),
            (
                CcPage::Detail(Detail::Wifi),
                3,
                1032,
                320,
                20 + 36 + 6 + 3 * 44 + 6 + 29,
            ),
            (
                CcPage::Detail(Detail::Bluetooth),
                0,
                1032,
                320,
                20 + 36 + 6 + 41 + 6 + 29,
            ),
        ];
        for (page, rows, cap, width, height) in cases {
            let size = estimate_size(&cc, page, &placed, rows, cap);
            assert_eq!(
                (size.width, size.height),
                (width, height),
                "{page:?} {rows}"
            );
        }
        assert_eq!(estimate_size(&cc, CcPage::Modules, &[], 0, 900).height, 20);
    }
}
