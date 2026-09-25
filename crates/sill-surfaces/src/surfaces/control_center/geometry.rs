//! The control center's grid and size (design/13 §13.3.7: width 320, two columns, gap 8,
//! padding 12, sliders spanning both columns; height fits content, at most
//! `output_h - bar_h - 16`, then scrolls), pure. Every number is a `control_center.*` or
//! `bar.*` key.
//!
//! The popup's size is estimated, not measured: shell-host's `PopupSize::FitContent` never
//! measures the document (G7), so the bar sizes its popup from [`estimate_size`] and repositions
//! it on a page change, as it does for its menus (`bar/geometry.rs`).

use shell_host::Size;
use sill_settings::{BarSettings, ControlCenterModule, ControlCenterSettings};

use super::page::CcPage;

/// How many grid columns a module takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Span {
    /// One column: a square-ish tile (Wi-Fi, Bluetooth, Focus).
    One,
    /// The whole row: sliders (Display, Sound), Now Playing, Appearance, Battery.
    Row,
}

/// A module's span.
pub fn span(module: ControlCenterModule) -> Span {
    let _ = module;
    todo!("M5 fill: control-center (c)")
}

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

/// `modules` laid out in order, `columns` wide: a `One` module takes the next free column of
/// the current row; a `Row` module starts a new row (leaving any free column of the current row
/// empty) and fills it.
pub fn place(modules: &[ControlCenterModule], columns: u16) -> Vec<Placed> {
    let _ = (modules, columns);
    todo!("M5 fill: control-center (c)")
}

/// The panel's greatest height on an output `output_height` tall.
pub fn height_cap(output_height: i32, bar: &BarSettings, cc: &ControlCenterSettings) -> i32 {
    let _ = (output_height, bar, cc);
    todo!("M5 fill: control-center (c)")
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
    let _ = (cc, page, placed, list_rows, cap);
    todo!("M5 fill: control-center (c)")
}

#[cfg(test)]
mod tests {
    use super::{Placed, Span, height_cap, place};
    use sill_settings::{BarSettings, ControlCenterModule, ControlCenterSettings};

    #[test]
    #[ignore = "M5 fill: control-center (c)"]
    fn the_default_modules_lay_out_in_two_columns() {
        use ControlCenterModule::{
            Appearance, Battery, Bluetooth, Display, Focus, NowPlaying, Sound, Wifi,
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
                (Sound, 3, 0, Span::Row),
                (NowPlaying, 4, 0, Span::Row),
                (Appearance, 5, 0, Span::Row),
                (Battery, 6, 0, Span::Row),
            ]
        );
    }

    #[test]
    #[ignore = "M5 fill: control-center (c)"]
    fn the_panel_stops_short_of_the_bottom_edge() {
        let (bar, cc) = (BarSettings::default(), ControlCenterSettings::default());
        // 1080 - bar 32 - margin 16.
        assert_eq!(height_cap(1080, &bar, &cc), 1032);
    }
}
