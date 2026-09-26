//! The emoji grid (M9 lanes c and d, FINDINGS "M9 freeze" F656): the Emoji section as cells,
//! recent first, arrow keys moving in two dimensions, Enter copying. Drawn by quire's
//! `EmojiGrid` (Q291). In the launcher the grid is a group of the palette
//! (`sections_view::palette_groups`, `PaletteGroup::grid`), where the field keeps the keyboard;
//! [`EmojiGrid`] is the same grid on its own, focusable, for a surface without a palette.
//!
//! Glyphs paint in colour from the system's COLRv1 face (Noto Color Emoji here; CBDT is out):
//! quire's grid carries its `ds-emoji-text` class, which applies the `--font-emoji` token. sill
//! never writes `font-family` (quire's lint refuses it). Lane c timed 300 cells scrolled at
//! 60 Hz on vello_cpu (release): p50 6.89 ms, p95 9.57 ms, max 12.95 ms (vello_hybrid: Q330).

use dioxus::prelude::*;
use ds::{EMOJI_CELL, EmojiCell, GridMove};
use sill_launcher::{Item, Subject};

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
    let cells: Vec<EmojiCell<RowRef>> = cells.iter().map(cell_of).collect();
    rsx! {
        ds::EmojiGrid::<RowRef> {
            cells,
            onpick,
            columns: u8::try_from(GRID_COLUMNS).unwrap_or(u8::MAX),
            cell: EMOJI_CELL,
            selected,
        }
    }
}

/// An emoji row as a cell: its glyph and name.
fn cell_of(item: &Item) -> EmojiCell<RowRef> {
    let (glyph, name) = match &item.subject {
        Subject::Emoji(emoji) => (emoji.glyph.clone(), emoji.name.clone()),
        _ => (item.title.clone(), item.subtitle.clone()),
    };
    EmojiCell {
        value: RowRef::of(item),
        glyph,
        name,
    }
}

/// Where Up, Down, Left and Right move a selection in a grid of `count` cells, `columns` wide:
/// quire's move (`ds::grid_step`, Q291): Left and Right walk the cells in reading order, wrapping
/// across rows, and stop at the first and last cell; Up and Down move a row and, past the top or
/// the bottom row, stay (a palette leaves the grid there instead).
pub fn grid_step(selected: usize, step: GridStep, count: usize, columns: usize) -> usize {
    let quire = match step {
        GridStep::Up => ds::GridStep::Up,
        GridStep::Down => ds::GridStep::Down,
        GridStep::Left => ds::GridStep::Left,
        GridStep::Right => ds::GridStep::Right,
    };
    let columns = u8::try_from(columns).unwrap_or(u8::MAX);
    match ds::grid_step(selected, quire, count, columns) {
        GridMove::To(at) => at,
        GridMove::Out(_) => selected.min(count.saturating_sub(1)),
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

    /// The freeze's table, with Right from a row's last cell wrapping to the next row's first
    /// (quire's grid, as the reference's character viewer; the coordinator's change of this
    /// frozen test, FINDINGS "M9 lane d").
    #[test]
    fn arrows_move_in_two_dimensions_and_stop_at_the_edges() {
        let cases: &[(usize, GridStep, usize)] = &[
            (0, GridStep::Right, 1),
            (0, GridStep::Left, 0),
            (0, GridStep::Down, 8),
            (3, GridStep::Up, 3),
            (8, GridStep::Up, 0),
            (7, GridStep::Right, 8),
            (8, GridStep::Left, 7),
            (12, GridStep::Right, 12),
            (12, GridStep::Down, 12),
            (4, GridStep::Down, 12),
        ];
        for &(from, step, want) in cases {
            assert_eq!(grid_step(from, step, 13, 8), want, "{from} {step:?}");
        }
    }
}
