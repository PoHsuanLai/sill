//! How the app switcher's row fits its output (design/13-BEHAVIOUR-menus-windows.md section
//! 13.3.5, "Overflow"): cells at their full size while the row fits `output_w - 64`; past that
//! the icons shrink to fit, down to a minimum; past that the row keeps the minimum and scrolls,
//! with the selection centred where it can be and the row's ends never pulled inside the view.
//! Pure: the numbers only, in whole logical pixels.

use ds::base::geometry::units::Px;

/// The panel's padding on each side (design/13 section 13.3.5, "padding 16").
pub const SWITCHER_PADDING: Px = Px(16.0);

/// What the panel leaves free of the output's width (design/13 section 13.3.5,
/// "`output_w - 64`"): 32 each side.
pub const SWITCHER_MARGIN: Px = Px(64.0);

/// The switcher's geometry from the settings (`switcher.icon_size_px`, `cell_size_px`,
/// `cell_gap_px`, `overflow_min_icon_px`; design/22-SETTINGS.md section 3.11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitcherMetrics {
    /// The icon at full size, 96.
    pub icon: Px,
    /// The cell at full size, 112: the icon and its inset, the selection's square.
    pub cell: Px,
    /// The gap between cells, 8.
    pub gap: Px,
    /// The least an icon shrinks to before the row scrolls instead, 48.
    pub min_icon: Px,
}

impl Default for SwitcherMetrics {
    /// The keys' defaults (design/22 section 3.11, proposed).
    fn default() -> Self {
        SwitcherMetrics {
            icon: Px(96.0),
            cell: Px(112.0),
            gap: Px(8.0),
            min_icon: Px(48.0),
        }
    }
}

/// The row as drawn: the icon and cell it settled on, the width of the view that shows it, and
/// how far the row is scrolled inside that view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitcherFit {
    /// The icon's side.
    pub icon: Px,
    /// The cell's side.
    pub cell: Px,
    /// The view's width: the whole row when it fits, else the room the output leaves.
    pub view: Px,
    /// How far the row is scrolled left inside the view; zero when it fits.
    pub shift: Px,
}

/// The fit of `count` cells with the one at `selected` selected, on an output `output` wide
/// (`None`: as wide as the row needs, never shrunk).
pub fn fit(
    count: usize,
    selected: usize,
    metrics: SwitcherMetrics,
    output: Option<Px>,
) -> SwitcherFit {
    let inset = (metrics.cell.0 - metrics.icon.0).max(0.0);
    let room =
        output.map(|width| (width.0 - SWITCHER_MARGIN.0 - 2.0 * SWITCHER_PADDING.0).max(0.0));
    let cells = count.max(1) as f32;
    let gaps = (cells - 1.0) * metrics.gap.0;
    let full = row(cells, metrics.cell.0, gaps);
    let cell = match room {
        Some(room) if full > room => {
            let shrunk = ((room - gaps) / cells).floor();
            shrunk.min(metrics.cell.0).max(metrics.min_icon.0 + inset)
        }
        _ => metrics.cell.0,
    };
    let icon = cell - inset;
    let width = row(cells, cell, gaps);
    let view = room.map_or(width, |room| width.min(room));
    let shift = if width > view {
        let at = selected.min(count.saturating_sub(1)) as f32;
        let centre = at * (cell + metrics.gap.0) + cell / 2.0;
        (centre - view / 2.0).clamp(0.0, width - view).round()
    } else {
        0.0
    };
    SwitcherFit {
        icon: Px(icon),
        cell: Px(cell),
        view: Px(view),
        shift: Px(shift),
    }
}

/// A row of `cells` cells `cell` wide with `gaps` between them.
fn row(cells: f32, cell: f32, gaps: f32) -> f32 {
    cells * cell + gaps
}

#[cfg(test)]
mod tests {
    use super::{SwitcherFit, SwitcherMetrics, fit};
    use ds::base::geometry::units::Px;

    fn at(count: usize, selected: usize, output: Option<f32>) -> SwitcherFit {
        fit(count, selected, SwitcherMetrics::default(), output.map(Px))
    }

    #[test]
    fn a_row_that_fits_keeps_its_full_size_and_does_not_scroll() {
        // Five cells: 5 x 112 + 4 x 8 = 592, inside 1440 - 64 - 32.
        let five = at(5, 1, Some(1440.0));
        assert_eq!(
            five,
            SwitcherFit {
                icon: Px(96.0),
                cell: Px(112.0),
                view: Px(592.0),
                shift: Px(0.0)
            }
        );
        assert_eq!(at(5, 4, None), five);
    }

    #[test]
    fn a_row_too_wide_shrinks_its_icons_first() {
        // Fourteen on 1440: room 1344; (1344 - 13 x 8) / 14 = 88.57, so cells of 88, icons 72.
        let fourteen = at(14, 1, Some(1440.0));
        assert_eq!((fourteen.cell, fourteen.icon), (Px(88.0), Px(72.0)));
        assert_eq!(fourteen.view, Px(14.0 * 88.0 + 13.0 * 8.0));
        assert_eq!(fourteen.shift, Px(0.0));
    }

    #[test]
    fn past_the_minimum_the_row_scrolls_to_keep_the_selection_in_view() {
        // Fourteen on 800: room 704 would need cells of 42, under the minimum 48 + 16, so cells
        // stay 64 and the row (14 x 64 + 13 x 8 = 1000) scrolls inside a 704 view.
        let cases = [(0, 0.0), (1, 0.0), (7, 184.0), (13, 296.0)];
        for (selected, shift) in cases {
            let row = at(14, selected, Some(800.0));
            assert_eq!(
                (row.cell, row.icon, row.view),
                (Px(64.0), Px(48.0), Px(704.0))
            );
            assert_eq!(row.shift, Px(shift), "selected {selected}");
            let left = selected as f32 * 72.0 - row.shift.0;
            assert!(
                left >= 0.0 && left + 64.0 <= row.view.0,
                "selected {selected} is in view"
            );
        }
    }

    #[test]
    fn no_apps_and_a_stray_selection_stay_sane() {
        assert_eq!(at(0, 0, Some(1440.0)).cell, Px(112.0));
        let stray = at(14, 99, Some(800.0));
        assert_eq!(stray.shift, Px(296.0));
    }
}
