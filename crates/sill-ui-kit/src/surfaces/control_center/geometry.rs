//! The control center's grid and cap (design/13 §13.3.7, design/29: width 320, two columns,
//! gap 10, padding 10, sliders spanning both columns; height fits content, at most
//! `output_h - bar_h - 16`, then scrolls), pure. Every number is a `control_center.*` or
//! `bar.*` key. The popup's size is shell-host's content fit, not computed here.

use sill_settings::{BarSettings, ControlCenterModule, ControlCenterSettings};

use super::modules::{Span, module};

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
}
