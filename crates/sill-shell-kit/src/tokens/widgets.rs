//! The widget grid's unit (design/20-SURFACES.md section 1.14): a cell's side and the gap
//! between cells, which a host sets on an element around its widgets (the notification center
//! writes 160 and 16). A `WidgetFrame` sizes itself from both.

use ds::base::word::Word;
use ds::style::tokens::token::Token;

/// One widget grid token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum WidgetGrid {
    /// `--widget-cell` (164 unless the host sets it): a small widget's side.
    #[token(name = "widget-cell", input = "--widget-cell-px", value = "164px")]
    Cell,
    /// `--widget-gap` (16): between two cells, so a medium widget is two
    /// cells and one gap wide.
    #[token(name = "widget-gap", input = "--widget-gap-px", value = "16px")]
    Gap,
}
