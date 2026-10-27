//! The bar's geometry tokens as the layout maths reads them: the bar's height
//! (`--shell-bar-h`, quire's `SHELL_SCALE.bar`) and the status items' box (`--bar-status-box`,
//! the `MenuBarItem` sheet's fallback). The maths needs them before the first layout, so
//! [`BarTokens::read`] takes the resolved length of a custom property
//! (`ds_blitz::Resolved::length` on the surface's document, read again when the `Styles` signal
//! changes) and a token a sheet does not write, or writes as `calc()`, is the shipped value. The
//! status glyph (`--bar-status-glyph`) is only drawn, never computed with, so it has no constant.

use ds::prelude::Px;
use ds::style::tokens::shell_scale::SHELL_SCALE;

/// `--bar-status-box`'s shipped value: a status item's height, and the tray's raster size.
pub const STATUS_BOX: u16 = 22;

/// The token values the bar's maths uses, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarTokens {
    /// The bar's height.
    pub height: u16,
    /// A status item's box.
    pub status_box: u16,
}

impl Default for BarTokens {
    fn default() -> Self {
        BarTokens {
            height: SHELL_SCALE.bar.0,
            status_box: STATUS_BOX,
        }
    }
}

impl BarTokens {
    /// The values `length` resolves (a property name to its length), the shipped value for each
    /// it does not.
    pub fn read(length: impl Fn(&str) -> Option<Px>) -> BarTokens {
        let shipped = BarTokens::default();
        let one = |name: &str, shipped: u16| {
            length(name).map_or(shipped, |px| {
                px.0.round().clamp(0.0, f32::from(u16::MAX)) as u16
            })
        };
        BarTokens {
            height: one("--shell-bar-h", shipped.height),
            status_box: one("--bar-status-box", shipped.status_box),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BarTokens, STATUS_BOX};
    use ds::prelude::Px;
    use ds::style::tokens::shell_scale::ShellSize;
    use ds::style::tokens::token::{Token, TokenScope};

    #[test]
    fn the_shipped_values_are_the_stylesheets() {
        let tokens = BarTokens::default();
        assert_eq!(
            ShellSize::Bar.css_value(TokenScope::BASE).as_str(),
            format!("{}px", tokens.height)
        );
        let sheet = include_str!("../../../sill-shell-kit/src/bar/menu_bar_item.css");
        assert!(sheet.contains(&format!("var(--bar-status-box,{STATUS_BOX}px)")));
    }

    #[test]
    fn a_token_a_sheet_writes_beats_the_shipped_value() {
        let wrote = |name: &str| (name == "--bar-status-box").then_some(Px(26.0));
        let tokens = BarTokens::read(wrote);
        assert_eq!(tokens.status_box, 26);
        assert_eq!(tokens.height, BarTokens::default().height);
    }
}
