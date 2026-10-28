//! The indicator's tuned token as the layer maths reads it: `--osd-margin`
//! (`sill_shell_kit::tokens::osd::OsdToken`), the card's gap from the bar's reserve and from the
//! output's right edge. The surface's margin is needed before the first layout, so
//! [`OsdTokens::read`] takes the resolved length of a custom property
//! (`ds_blitz::Resolved::length` on the surface's document, read again when the `Styles` signal
//! changes) and a token a sheet does not write, or writes as `calc()`, is [`MARGIN`]. The
//! constant and the token's default are the only two copies; a test compares them.

use ds::prelude::Px;
use sill_shell_kit::tokens::osd::OsdToken;

/// `--osd-margin`'s shipped value.
pub const MARGIN: u16 = 24;

/// The token values the indicator's maths uses, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsdTokens {
    /// The card's gap from the edge it is anchored to.
    pub margin: u16,
}

impl Default for OsdTokens {
    fn default() -> Self {
        OsdTokens { margin: MARGIN }
    }
}

impl OsdTokens {
    /// The values `length` resolves (a property name to its length), the shipped constant for
    /// each it does not.
    pub fn read(length: impl Fn(&str) -> Option<Px>) -> OsdTokens {
        let margin = length(OsdToken::Margin.input().as_str()).map_or(MARGIN, |px| {
            px.0.round().clamp(0.0, f32::from(u16::MAX)) as u16
        });
        OsdTokens { margin }
    }
}

impl sill_ui_kit::hooks::ReadTokens for OsdTokens {
    fn read_tokens(length: &dyn Fn(&str) -> Option<Px>) -> Self {
        Self::read(length)
    }
}

#[cfg(test)]
mod tests {
    use super::{MARGIN, OsdTokens};
    use ds::prelude::Px;
    use ds::style::tokens::token::TokenScope;
    use sill_shell_kit::tokens::osd::OsdToken;

    #[test]
    fn the_constant_is_the_tokens_default() {
        assert_eq!(
            OsdToken::Margin.fallback(TokenScope::BASE),
            format!("{MARGIN}px").as_str()
        );
    }

    #[test]
    fn a_token_a_sheet_writes_beats_the_constant() {
        let wrote = |name: &str| (name == "--osd-margin-px").then_some(Px(40.0));
        assert_eq!(OsdTokens::read(wrote).margin, 40);
        assert_eq!(OsdTokens::read(|_| None), OsdTokens::default());
    }
}
