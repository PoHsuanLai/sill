//! The OSD card's margin token: the gap from the bar's reserve to the card at the top right, or
//! from the dock's at the bottom centre. A tuned token a look or the person's style sheet may
//! move; the indicator's layer surface reads it before layout (`sill_overlays::osd::tokens`).

use ds::base::word::Word;
use ds::style::tokens::token::Token;

/// The OSD card's tuned token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum OsdToken {
    /// `--osd-margin`: the card's gap from the edge it is anchored to (24).
    #[token(name = "osd-margin", input = "--osd-margin-px", value = "24px")]
    Margin,
}
