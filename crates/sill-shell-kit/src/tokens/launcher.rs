//! The launcher panel's placement tokens (design/13 §13.5: `top = max(bar_h + gap, round(0.4 H -
//! bias))`), each a tuned token a look or the person's style sheet may move. The panel's layer
//! surface is placed from them before it has a layout (`sill_launcher_ui`'s `LauncherTokens`).

use ds::base::word::Word;
use ds::style::tokens::token::Token;

/// One launcher placement token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum LauncherToken {
    /// `--sill-launcher-top-bar-gap`: the least space between the bar and the panel (24).
    #[token(
        name = "sill-launcher-top-bar-gap",
        input = "--sill-launcher-top-bar-gap-px",
        value = "24px"
    )]
    TopBarGap,
    /// `--sill-launcher-top-centre-bias`: how far above the output's 40% line the panel sits
    /// (230).
    #[token(
        name = "sill-launcher-top-centre-bias",
        input = "--sill-launcher-top-centre-bias-px",
        value = "230px"
    )]
    TopCentreBias,
}
