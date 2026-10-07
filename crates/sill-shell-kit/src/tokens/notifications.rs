//! The notification parts' geometry tokens (design/22-SETTINGS.md section 3.12,
//! design/13 section 13.3.6): the banner's width, floor, padding and icon, the close button, the
//! gap between stacked banners, the offset of a group's layers and the notification center's
//! width, each a tuned token its `notifications.*` key moves through one inline write
//! ([`NotificationMetrics::style_attr`]) on any element around the banners or the center.

use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::style::tokens::token::Token;
use ds::style::tokens::tuned::px;

/// One notification geometry token, each a tuned token its `notifications.*` key moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = tuned)]
pub enum NotificationToken {
    /// `--notifications-banner-width` (`notifications.banner_width_px`, 360).
    #[token(
        name = "notifications-banner-width",
        input = "--notifications-banner-width-px",
        value = "360px"
    )]
    BannerWidth,
    /// `--notifications-banner-min-height` (`notifications.banner_min_height_px`, 64).
    #[token(
        name = "notifications-banner-min-height",
        input = "--notifications-banner-min-height-px",
        value = "64px"
    )]
    BannerMinHeight,
    /// `--notifications-banner-padding` (`notifications.banner_padding_px`, 12).
    #[token(
        name = "notifications-banner-padding",
        input = "--notifications-banner-padding-px",
        value = "12px"
    )]
    BannerPadding,
    /// `--notifications-icon` (`notifications.icon_px`, 32): the app icon's side.
    #[token(
        name = "notifications-icon",
        input = "--notifications-icon-px",
        value = "32px"
    )]
    Icon,
    /// `--notifications-close` (`notifications.close_button_px`, 18): the close button's diameter.
    #[token(
        name = "notifications-close",
        input = "--notifications-close-px",
        value = "18px"
    )]
    Close,
    /// `--notifications-stack-gap` (`notifications.stack_gap_px`, 8): between two banners.
    #[token(
        name = "notifications-stack-gap",
        input = "--notifications-stack-gap-px",
        value = "8px"
    )]
    StackGap,
    /// `--notifications-group-offset` (`notifications.group_offset_px`, 4): how far each of a
    /// group's layers shows below the one above it.
    #[token(
        name = "notifications-group-offset",
        input = "--notifications-group-offset-px",
        value = "4px"
    )]
    GroupOffset,
    /// `--notifications-center-width` (`notifications.center_width_px`, 384).
    #[token(
        name = "notifications-center-width",
        input = "--notifications-center-width-px",
        value = "384px"
    )]
    CenterWidth,
}

/// The notification geometry from the settings, written as the tokens' inputs on any element
/// around the banners or the center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotificationMetrics {
    /// A banner's width, 360.
    pub banner_width: Px,
    /// A banner's least height, 64.
    pub banner_min_height: Px,
    /// A banner's padding, 12.
    pub banner_padding: Px,
    /// The app icon's side, 32.
    pub icon: Px,
    /// The close button's diameter, 18.
    pub close: Px,
    /// Between two stacked banners, 8.
    pub stack_gap: Px,
    /// Each group layer's offset below the one above it, 4.
    pub group_offset: Px,
    /// The notification center's width, 384 (the key's range is 280 to 600).
    pub center_width: Px,
}

impl Default for NotificationMetrics {
    /// The keys' defaults (design/22 section 3.12).
    fn default() -> Self {
        NotificationMetrics {
            banner_width: Px(360.0),
            banner_min_height: Px(64.0),
            banner_padding: Px(12.0),
            icon: Px(32.0),
            close: Px(18.0),
            stack_gap: Px(8.0),
            group_offset: Px(4.0),
            center_width: Px(384.0),
        }
    }
}

impl NotificationMetrics {
    /// Every input, inline: `--notifications-banner-width-px:360px;…`. The center's width is
    /// held to its key's 280 to 600; every other length to 0 to 999.
    pub fn style_attr(&self) -> String {
        let length = |value: Px| px(value.0.round().clamp(0.0, 999.0) as u16);
        [
            NotificationToken::BannerWidth.write(&length(self.banner_width)),
            NotificationToken::BannerMinHeight.write(&length(self.banner_min_height)),
            NotificationToken::BannerPadding.write(&length(self.banner_padding)),
            NotificationToken::Icon.write(&length(self.icon)),
            NotificationToken::Close.write(&length(self.close)),
            NotificationToken::StackGap.write(&length(self.stack_gap)),
            NotificationToken::GroupOffset.write(&length(self.group_offset)),
            NotificationToken::CenterWidth
                .write(&length(Px(self.center_width.0.clamp(280.0, 600.0)))),
        ]
        .concat()
    }
}

#[cfg(test)]
mod tests {
    use super::{NotificationMetrics, NotificationToken};
    use ds::base::geometry::units::Px;
    use ds::base::word::Word;
    use ds::style::tokens::token::TokenScope;

    #[test]
    fn the_defaults_write_what_the_stylesheet_falls_back_to() {
        let written = NotificationMetrics::default().style_attr();
        let want: String = NotificationToken::ALL
            .iter()
            .map(|token| token.write(token.fallback(TokenScope::BASE)))
            .collect();
        assert_eq!(written, want);
    }

    #[test]
    fn the_center_is_held_to_its_keys_range() {
        let wide = NotificationMetrics {
            center_width: Px(900.0),
            ..NotificationMetrics::default()
        };
        assert!(
            wide.style_attr()
                .contains("--notifications-center-width-px:600px;")
        );
        let narrow = NotificationMetrics {
            center_width: Px(100.0),
            ..NotificationMetrics::default()
        };
        assert!(
            narrow
                .style_attr()
                .contains("--notifications-center-width-px:280px;")
        );
    }
}
