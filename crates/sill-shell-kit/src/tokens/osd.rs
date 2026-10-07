//! The OSD card's margin token (design/22-SETTINGS.md section 3.16
//! `osd.margin_px`): the gap from the bar's reserve to the card at the top right, or from the
//! dock's at the bottom centre. A tuned token so the setting reaches the card through one
//! inline write ([`OsdMetrics::style_attr`]) on any element around it, as the dock's geometry.

use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::style::tokens::token::Token;
use ds::style::tokens::tuned::px;

/// The OSD card's tuned token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum OsdToken {
    /// `--osd-margin`: the card's gap from the edge it is anchored to (`osd.margin_px`, 24).
    #[token(name = "osd-margin", input = "--osd-margin-px", value = "24px")]
    Margin,
}

/// The OSD's geometry from the settings, written as the tokens' inputs on any element around the
/// card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsdMetrics {
    /// The card's gap from the bar's reserve (top right) or the dock's (bottom centre), 24.
    pub margin: Px,
}

impl Default for OsdMetrics {
    /// The key's default (design/22 section 3.16, proposed 2026-09-25).
    fn default() -> Self {
        OsdMetrics { margin: Px(24.0) }
    }
}

impl OsdMetrics {
    /// Every input, inline: `--osd-margin-px:24px;`. A margin outside the key's `0..=400` is
    /// held to it.
    pub fn style_attr(&self) -> String {
        OsdToken::Margin.write(&px(self.margin.0.round().clamp(0.0, 400.0) as u16))
    }
}

#[cfg(test)]
mod tests {
    use super::{OsdMetrics, OsdToken};
    use ds::base::geometry::units::Px;
    use ds::base::word::Word;
    use ds::style::tokens::token::TokenScope;

    #[test]
    fn the_default_writes_what_the_stylesheet_falls_back_to() {
        let written = OsdMetrics::default().style_attr();
        for token in OsdToken::ALL.iter().copied() {
            assert_eq!(written, token.write(token.fallback(TokenScope::BASE)));
        }
    }

    #[test]
    fn the_margin_is_held_to_the_keys_range() {
        let far = OsdMetrics { margin: Px(900.0) };
        assert_eq!(far.style_attr(), "--osd-margin-px:400px;");
        let near = OsdMetrics { margin: Px(-3.0) };
        assert_eq!(near.style_attr(), "--osd-margin-px:0px;");
    }
}
