//! The dock's geometry tokens as the layout maths reads them. Each is a tuned token in
//! `sill_shell_kit::tokens::dock::DockToken` (`--dock-gap`, `--dock-pad`,
//! `--sill-dock-pill-radius`, `--sill-dock-edge-clamp`), a look or the person's style sheet may
//! move it, and the maths needs the number before the first layout. [`DockTokens::read`] takes
//! the resolved length of a custom property (`ds_blitz::Resolved::length` on the surface's
//! document, read again when the `Styles` signal changes); a token a sheet does not write, or
//! writes as `calc()`, is the shipped constant below. Those constants and the token defaults are
//! the only two copies; a test compares them.

use ds::prelude::Px;
use sill_shell_kit::tokens::dock::DockToken;

/// `--dock-gap`'s shipped value: between two tiles.
pub const GAP: u16 = 8;
/// `--dock-pad`'s shipped value: the pill's padding on every side.
pub const PAD: u16 = 6;
/// `--sill-dock-pill-radius`'s shipped value: the pill's squircle corner.
pub const PILL_RADIUS: u16 = 22;
/// `--sill-dock-edge-clamp`'s shipped value: how close the pill gets to the output's sides.
pub const EDGE_CLAMP: u16 = 8;

/// The token values the dock's maths uses, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockTokens {
    /// Between two tiles.
    pub gap: u16,
    /// The pill's padding on every side.
    pub pad: u16,
    /// The pill's nominal corner radius.
    pub pill_radius: u16,
    /// How close the pill gets to the output's sides.
    pub edge_clamp: u16,
}

impl Default for DockTokens {
    fn default() -> Self {
        DockTokens {
            gap: GAP,
            pad: PAD,
            pill_radius: PILL_RADIUS,
            edge_clamp: EDGE_CLAMP,
        }
    }
}

impl DockTokens {
    /// The values `length` resolves (a property name to its length), the shipped constant for
    /// each it does not.
    pub fn read(length: impl Fn(&str) -> Option<Px>) -> DockTokens {
        let one = |token: DockToken, shipped: u16| {
            length(token.input().as_str()).map_or(shipped, |px| {
                px.0.round().clamp(0.0, f32::from(u16::MAX)) as u16
            })
        };
        DockTokens {
            gap: one(DockToken::Gap, GAP),
            pad: one(DockToken::Pad, PAD),
            pill_radius: one(DockToken::PillRadius, PILL_RADIUS),
            edge_clamp: one(DockToken::EdgeClamp, EDGE_CLAMP),
        }
    }
}

impl sill_ui_kit::hooks::ReadTokens for DockTokens {
    fn read_tokens(length: &dyn Fn(&str) -> Option<Px>) -> Self {
        Self::read(length)
    }
}

#[cfg(test)]
mod tests {
    use super::{DockTokens, EDGE_CLAMP, GAP, PAD, PILL_RADIUS};
    use ds::prelude::Px;
    use ds::style::tokens::token::TokenScope;
    use sill_shell_kit::tokens::dock::DockToken;

    #[test]
    fn the_constants_are_the_tokens_defaults() {
        let cases = [
            (DockToken::Gap, GAP),
            (DockToken::Pad, PAD),
            (DockToken::PillRadius, PILL_RADIUS),
            (DockToken::EdgeClamp, EDGE_CLAMP),
        ];
        for (token, shipped) in cases {
            assert_eq!(
                token.fallback(TokenScope::BASE),
                format!("{shipped}px").as_str()
            );
        }
    }

    #[test]
    fn a_token_a_sheet_writes_beats_the_constant_and_the_rest_keep_it() {
        let wrote = |name: &str| (name == "--dock-gap-px").then_some(Px(12.0));
        let tokens = DockTokens::read(wrote);
        assert_eq!(tokens.gap, 12);
        assert_eq!(tokens.pad, PAD);
        assert_eq!(DockTokens::read(|_| None), DockTokens::default());
    }
}
