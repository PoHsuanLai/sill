//! The card's numbers in quire's words: the notification tokens the layer maths and the icon's
//! raster read ([`NotificationTokens`]), the swipe thresholds as a `SwipeMetrics`, and the
//! icon's side. The banner and center widths, the icon and the banner column's place are tuned
//! tokens (`NotificationToken`) a look or the person's style sheet may move; the surfaces need
//! them before the first layout, so [`NotificationTokens::read`] takes the resolved length of a
//! custom property (`ds_blitz::Resolved::length` on the surface's document, read again when the
//! `Styles` signal changes) and a token a sheet does not write, or writes as `calc()`, is the
//! shipped constant. The constants and the token defaults are the only two copies; a test
//! compares them.

use ds::motion::swipe::{Speed, SwipeMetrics};
use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds_settings::Fraction;
use sill_shell_kit::tokens::notifications::NotificationToken;

/// `--notifications-banner-width`'s shipped value.
pub const BANNER_WIDTH: u16 = 320;
/// `--notifications-center-width`'s shipped value.
pub const CENTER_WIDTH: u16 = 320;
/// `--notifications-icon`'s shipped value: the app icon's side.
pub const ICON: u16 = 28;
/// `--sill-notifications-position-right`'s shipped value.
pub const POSITION_RIGHT: u16 = 8;
/// `--sill-notifications-position-top-gap`'s shipped value.
pub const POSITION_TOP_GAP: u16 = 8;

/// The token values the notification surfaces use, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationTokens {
    /// A banner's width.
    pub banner_width: u16,
    /// The notification center's width.
    pub center_width: u16,
    /// The app icon's side.
    pub icon: u16,
    /// A banner's gap from the output's right edge.
    pub position_right: u16,
    /// A banner's gap from the bar's reserve.
    pub position_top_gap: u16,
}

impl Default for NotificationTokens {
    fn default() -> Self {
        NotificationTokens {
            banner_width: BANNER_WIDTH,
            center_width: CENTER_WIDTH,
            icon: ICON,
            position_right: POSITION_RIGHT,
            position_top_gap: POSITION_TOP_GAP,
        }
    }
}

impl NotificationTokens {
    /// The values `length` resolves (a property name to its length), the shipped constant for
    /// each it does not.
    pub fn read(length: impl Fn(&str) -> Option<Px>) -> NotificationTokens {
        let one = |token: NotificationToken, shipped: u16| {
            length(token.input().as_str()).map_or(shipped, |px| {
                px.0.round().clamp(0.0, f32::from(u16::MAX)) as u16
            })
        };
        NotificationTokens {
            banner_width: one(NotificationToken::BannerWidth, BANNER_WIDTH),
            center_width: one(NotificationToken::CenterWidth, CENTER_WIDTH),
            icon: one(NotificationToken::Icon, ICON),
            position_right: one(NotificationToken::PositionRight, POSITION_RIGHT),
            position_top_gap: one(NotificationToken::PositionTopGap, POSITION_TOP_GAP),
        }
    }
}

/// How far a card is swiped to dismiss it, in px.
const SWIPE_DISMISS_PX: f32 = 80.0;
/// How fast a swipe dismisses from any distance, in px/s.
const SWIPE_DISMISS_VELOCITY_PX_S: f32 = 600.0;
/// How far a leftward drag moves the card, in thousandths (250 is 0.25).
const SWIPE_DAMPING: Fraction = Fraction(250);

/// quire's swipe thresholds for a card.
pub fn swipe_metrics() -> SwipeMetrics {
    SwipeMetrics {
        dismiss: Px(SWIPE_DISMISS_PX),
        velocity: Speed(SWIPE_DISMISS_VELOCITY_PX_S),
        damping: SWIPE_DAMPING,
    }
}

/// The app icon's side (`--notifications-icon`, 28), held to what an `IconPx` holds.
pub fn icon_size(tokens: &NotificationTokens) -> IconPx {
    IconPx(u8::try_from(tokens.icon).unwrap_or(u8::MAX))
}

impl crate::hooks::ReadTokens for NotificationTokens {
    fn read_tokens(length: &dyn Fn(&str) -> Option<Px>) -> Self {
        Self::read(length)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BANNER_WIDTH, CENTER_WIDTH, ICON, NotificationTokens, POSITION_RIGHT, POSITION_TOP_GAP,
        icon_size, swipe_metrics,
    };
    use ds::motion::swipe::SwipeMetrics;
    use ds::prelude::Px;
    use ds::style::icon::render::IconPx;
    use ds::style::tokens::token::TokenScope;
    use sill_shell_kit::tokens::notifications::NotificationToken;

    #[test]
    fn the_constants_are_the_tokens_defaults() {
        let cases = [
            (NotificationToken::BannerWidth, BANNER_WIDTH),
            (NotificationToken::CenterWidth, CENTER_WIDTH),
            (NotificationToken::Icon, ICON),
            (NotificationToken::PositionRight, POSITION_RIGHT),
            (NotificationToken::PositionTopGap, POSITION_TOP_GAP),
        ];
        for (token, shipped) in cases {
            assert_eq!(
                token.fallback(TokenScope::BASE),
                format!("{shipped}px").as_str()
            );
        }
    }

    #[test]
    fn the_defaults_are_quires_and_a_written_token_reaches_the_icon() {
        let defaults = NotificationTokens::default();
        assert_eq!(swipe_metrics(), SwipeMetrics::default());
        assert_eq!(icon_size(&defaults), IconPx(28));
        let wrote = |name: &str| (name == "--notifications-icon-px").then_some(Px(400.0));
        let big = NotificationTokens::read(wrote);
        assert_eq!(big.icon, 400);
        assert_eq!(icon_size(&big), IconPx(255), "held to what an IconPx holds");
        assert_eq!(NotificationTokens::read(|_| None), defaults);
    }
}
