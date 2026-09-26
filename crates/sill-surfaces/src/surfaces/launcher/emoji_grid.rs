//! The emoji grid (M9 lanes c and d, FINDINGS "M9 freeze" F656): the Emoji section as cells,
//! recent first, arrow keys moving in two dimensions, Enter copying. Drawn by quire's
//! `EmojiGrid` (Q291) once it lands; the freeze's component draws nothing (never local markup).
//!
//! Glyphs paint in colour from the system's COLRv1 face (Noto Color Emoji here; CBDT is out):
//! the grid and every cell carry quire's `ds-emoji` class, which applies its `--font-emoji`
//! token (quire m9-parts). sill never writes `font-family` (quire's lint refuses it). Open risk
//! (F656): a scrolling grid of a few hundred colour glyphs at 60 Hz on vello_hybrid is untimed;
//! lane c times it before the grid relies on it.

use dioxus::prelude::*;
use sill_launcher::Item;

use crate::surfaces::launcher::entries::RowRef;

/// Cells per grid row in the launcher's results column (proposed: 8 cells of 56 px in 540).
pub const GRID_COLUMNS: usize = 8;

/// `cells` (emoji rows, best first) as a grid, `selected` highlighted; a pick names the row.
#[component]
pub fn EmojiGrid(
    cells: Vec<Item>,
    selected: Option<usize>,
    onpick: EventHandler<RowRef>,
) -> Element {
    let _ = (&cells, selected, &onpick);
    rsx! {}
}

/// Where Up, Down, Left and Right move a selection in a grid of `count` cells, `columns` wide
/// (clamped at the edges, design/06 §2.3: no wrap). Lane d fills it.
pub fn grid_step(selected: usize, step: GridStep, count: usize, columns: usize) -> usize {
    let columns = columns.max(1);
    let last = count.saturating_sub(1);
    let at = selected.min(last);
    match step {
        GridStep::Up => at.checked_sub(columns).unwrap_or(at),
        GridStep::Down => Some(at + columns).filter(|&next| next <= last).unwrap_or(at),
        GridStep::Left if at % columns > 0 => at - 1,
        GridStep::Right if at % columns + 1 < columns && at < last => at + 1,
        GridStep::Left | GridStep::Right => at,
    }
}

/// A move in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridStep {
    Up,
    Down,
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::{GridStep, grid_step};

    #[test]
    fn arrows_move_in_two_dimensions_and_stop_at_the_edges() {
        let cases: &[(usize, GridStep, usize)] = &[
            (0, GridStep::Right, 1),
            (0, GridStep::Left, 0),
            (0, GridStep::Down, 8),
            (3, GridStep::Up, 3),
            (8, GridStep::Up, 0),
            (7, GridStep::Right, 7),
            (12, GridStep::Down, 12),
            (4, GridStep::Down, 12),
        ];
        for &(from, step, want) in cases {
            assert_eq!(grid_step(from, step, 13, 8), want, "{from} {step:?}");
        }
    }
}
