//! The card's numbers from `notifications.*` (design/22 §3.12) in quire's words: the eight
//! geometry keys as a `NotificationMetrics` (written once on a wrapper around the banners or the
//! center, as `--notifications-*` tokens), the swipe keys as a `SwipeMetrics`, and the icon's
//! side.

use ds::motion::swipe::{Speed, SwipeMetrics};
use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds_settings::Fraction;
use sill_settings::NotificationsSettings;
use sill_shell_kit::tokens::notifications::NotificationMetrics;

/// The geometry keys as quire's tokens; the rest of the card's sizes are quire's own tokens.
pub fn notification_metrics(n: &NotificationsSettings) -> NotificationMetrics {
    let px = |value: ds_settings::Px| Px(f32::from(value.0));
    NotificationMetrics {
        banner_width: px(n.banner_width_px),
        icon: px(n.icon_px),
        center_width: px(n.center_width_px),
        ..NotificationMetrics::default()
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

/// The app icon's side (`notifications.icon_px`, 28), held to what an `IconPx` holds.
pub fn icon_size(n: &NotificationsSettings) -> IconPx {
    IconPx(u8::try_from(n.icon_px.0).unwrap_or(u8::MAX))
}

#[cfg(test)]
mod tests {
    use super::{icon_size, notification_metrics, swipe_metrics};
    use ds::motion::swipe::SwipeMetrics;
    use ds::style::icon::render::IconPx;
    use ds_settings::Px;
    use sill_settings::NotificationsSettings;
    use sill_shell_kit::tokens::notifications::NotificationMetrics;

    #[test]
    fn the_defaults_are_quires_and_the_keys_reach_it() {
        let n = NotificationsSettings::default();
        let cases = [
            (
                n.clone(),
                NotificationMetrics::default(),
                SwipeMetrics::default(),
                IconPx(28),
            ),
            (
                NotificationsSettings {
                    banner_width_px: Px(420),
                    center_width_px: Px(500),
                    icon_px: Px(400),
                    ..n
                },
                NotificationMetrics {
                    banner_width: ds::prelude::Px(420.0),
                    center_width: ds::prelude::Px(500.0),
                    icon: ds::prelude::Px(400.0),
                    ..NotificationMetrics::default()
                },
                SwipeMetrics::default(),
                IconPx(255),
            ),
        ];
        for (settings, metrics, swipe, icon) in cases {
            assert_eq!(notification_metrics(&settings), metrics);
            assert_eq!(swipe_metrics(), swipe);
            assert_eq!(icon_size(&settings), icon);
        }
    }
}
