//! The launcher's placement tokens as the layer maths reads them. Each is a tuned token in
//! `sill_shell_kit::tokens::launcher::LauncherToken` (`--sill-launcher-top-bar-gap`,
//! `--sill-launcher-top-centre-bias`); the panel's place is needed before the first layout, so
//! [`LauncherTokens::read`] takes the resolved length of a custom property
//! (`ds_blitz::Resolved::length` on the surface's document, read again when the `Styles` signal
//! changes) and a token a sheet does not write, or writes as `calc()`, is the shipped constant.
//! The constants and the token defaults are the only two copies; a test compares them. The
//! bar's height is quire's `SHELL_SCALE.bar` (`--shell-bar-h`).

use ds::prelude::Px;
use ds::style::tokens::shell_scale::SHELL_SCALE;
use sill_shell_kit::tokens::launcher::LauncherToken;

/// `--sill-launcher-top-bar-gap`'s shipped value.
pub const TOP_BAR_GAP: u16 = 24;
/// `--sill-launcher-top-centre-bias`'s shipped value.
pub const TOP_CENTRE_BIAS: u16 = 230;

/// The token values the panel's placement uses, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LauncherTokens {
    /// The bar's height.
    pub bar_height: u16,
    /// The least space between the bar and the panel.
    pub top_bar_gap: u16,
    /// How far above the 40% line the panel sits.
    pub top_centre_bias: u16,
}

impl Default for LauncherTokens {
    fn default() -> Self {
        LauncherTokens {
            bar_height: SHELL_SCALE.bar.0,
            top_bar_gap: TOP_BAR_GAP,
            top_centre_bias: TOP_CENTRE_BIAS,
        }
    }
}

impl LauncherTokens {
    /// The values `length` resolves (a property name to its length), the shipped value for each
    /// it does not.
    pub fn read(length: impl Fn(&str) -> Option<Px>) -> LauncherTokens {
        let shipped = LauncherTokens::default();
        let whole = |px: Px| px.0.round().clamp(0.0, f32::from(u16::MAX)) as u16;
        let one = |token: LauncherToken, shipped: u16| {
            length(token.input().as_str()).map_or(shipped, whole)
        };
        LauncherTokens {
            bar_height: length("--shell-bar-h").map_or(shipped.bar_height, whole),
            top_bar_gap: one(LauncherToken::TopBarGap, TOP_BAR_GAP),
            top_centre_bias: one(LauncherToken::TopCentreBias, TOP_CENTRE_BIAS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LauncherTokens, TOP_BAR_GAP, TOP_CENTRE_BIAS};
    use ds::prelude::Px;
    use ds::style::tokens::token::TokenScope;
    use sill_shell_kit::tokens::launcher::LauncherToken;

    #[test]
    fn the_constants_are_the_tokens_defaults() {
        for (token, shipped) in [
            (LauncherToken::TopBarGap, TOP_BAR_GAP),
            (LauncherToken::TopCentreBias, TOP_CENTRE_BIAS),
        ] {
            assert_eq!(
                token.fallback(TokenScope::BASE),
                format!("{shipped}px").as_str()
            );
        }
    }

    #[test]
    fn a_token_a_sheet_writes_beats_the_constant() {
        let wrote = |name: &str| (name == "--sill-launcher-top-bar-gap-px").then_some(Px(40.0));
        let tokens = LauncherTokens::read(wrote);
        assert_eq!(tokens.top_bar_gap, 40);
        assert_eq!(tokens.top_centre_bias, TOP_CENTRE_BIAS);
        assert_eq!(LauncherTokens::read(|_| None), LauncherTokens::default());
    }
}
