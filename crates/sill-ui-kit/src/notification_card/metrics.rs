//! The card's numbers from `notifications.*` (design/22 §3.12) in quire's words: the eight
//! geometry keys as a `NotificationMetrics` (written once on a wrapper around the banners or the
//! center, as `--notifications-*` tokens), the swipe keys as a `SwipeMetrics`, and the icon's
//! side.

use ds::motion::swipe::{Speed, SwipeMetrics};
use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds_shell::tokens::notifications::NotificationMetrics;
use sill_settings::NotificationsSettings;

/// The geometry keys as quire's tokens.
pub fn notification_metrics(n: &NotificationsSettings) -> NotificationMetrics {
    let px = |value: ds_settings::Px| Px(f32::from(value.0));
    NotificationMetrics {
        banner_width: px(n.banner_width_px),
        banner_min_height: px(n.banner_min_height_px),
        banner_padding: px(n.banner_padding_px),
        icon: px(n.icon_px),
        close: px(n.close_button_px),
        stack_gap: px(n.stack_gap_px),
        group_offset: px(n.group_offset_px),
        center_width: px(n.center_width_px),
    }
}

/// The swipe keys as quire's swipe thresholds.
pub fn swipe_metrics(n: &NotificationsSettings) -> SwipeMetrics {
    SwipeMetrics {
        dismiss: Px(f32::from(n.swipe_dismiss_px.0)),
        velocity: Speed(n.swipe_dismiss_velocity_px_s.0),
        damping: n.swipe_damping,
    }
}

/// The app icon's side (`notifications.icon_px`, 32), held to what an `IconPx` holds.
pub fn icon_size(n: &NotificationsSettings) -> IconPx {
    IconPx(u8::try_from(n.icon_px.0).unwrap_or(u8::MAX))
}

#[cfg(test)]
mod tests {
    use super::{icon_size, notification_metrics, swipe_metrics};
    use ds::motion::swipe::SwipeMetrics;
    use ds::style::icon::render::IconPx;
    use ds_settings::Px;
    use ds_shell::tokens::notifications::NotificationMetrics;
    use sill_settings::NotificationsSettings;

    #[test]
    fn the_defaults_are_quires_and_the_keys_reach_it() {
        let n = NotificationsSettings::default();
        let cases = [
            (
                n.clone(),
                NotificationMetrics::default(),
                SwipeMetrics::default(),
                IconPx(32),
            ),
            (
                NotificationsSettings {
                    banner_width_px: Px(420),
                    center_width_px: Px(500),
                    icon_px: Px(400),
                    swipe_dismiss_px: Px(120),
                    ..n
                },
                NotificationMetrics {
                    banner_width: ds::prelude::Px(420.0),
                    center_width: ds::prelude::Px(500.0),
                    icon: ds::prelude::Px(400.0),
                    ..NotificationMetrics::default()
                },
                SwipeMetrics {
                    dismiss: ds::prelude::Px(120.0),
                    ..SwipeMetrics::default()
                },
                IconPx(255),
            ),
        ];
        for (settings, metrics, swipe, icon) in cases {
            assert_eq!(notification_metrics(&settings), metrics);
            assert_eq!(swipe_metrics(&settings), swipe);
            assert_eq!(icon_size(&settings), icon);
        }
    }
}
