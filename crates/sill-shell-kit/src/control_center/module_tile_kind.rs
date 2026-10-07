//! The words a `ModuleTile` is described in: how many grid columns it takes.

use ds::base::word::Word;

/// How many of the control center's grid columns a tile takes (design/13 section 13.3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum TileSpan {
    /// One column.
    #[default]
    Half,
    /// Every column (`grid-column:1 / -1`), as a slider module.
    Full,
}
