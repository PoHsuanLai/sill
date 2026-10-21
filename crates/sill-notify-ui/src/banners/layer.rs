//! The banners' layer surface (design/20 §1.6, design/13 §13.3.6): Overlay, a column down the
//! output's right edge (`TOP | RIGHT | BOTTOM`, so a stack of any height fits and
//! `notifications.banner_position` only moves the cards inside it), exclusive zone `Auto` (clear
//! of the bar's reserve above and the dock's below, as the OSD), no keyboard until `sill focus-latest` asks (the surface then sets `Exclusive`, `io.rs`).
//!
//! Input and blur start empty: the surface sets both to its cards' element regions
//! (`elements.rs`, through `shell_host::use_surface`) as they enter and leave, so the
//! transparent rest of the column never takes a click or blurs what is under it (design/20:
//! "per toast `Element("toast-<id>")`; input same", which a single `RegionSpec::Element` cannot
//! say for several cards).
//!
//! Spawned before the OSD, so on the same Overlay layer the OSD draws over a banner.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::{NotificationsSettings, ShellFile};

use sill_ui_kit::surface::namespace;

/// Transparent room around the column for the Toast material's shadow (the OSD's 40).
pub const ROOM: u32 = 40;

/// The card corner (design/13 §13.3.6: radius 16), which the regions follow.
pub const CARD_RADIUS: i32 = 16;

/// The margins that put a card `position_right_px` from the output's right edge and
/// `position_top_gap_px` from the bar's reserve (or, at the bottom, from the dock's).
pub fn margin(n: &NotificationsSettings) -> Margin {
    let room = i32::try_from(ROOM).unwrap_or(i32::MAX);
    let gap = i32::from(n.position_top_gap_px.0) - room;
    Margin {
        top: gap,
        right: i32::from(n.position_right_px.0) - room,
        bottom: gap,
        left: 0,
    }
}

/// The banner column on `output`.
pub fn layer(output: &OutputInfo, settings: &ShellFile) -> LayerConfig {
    let n = &settings.notifications;
    LayerConfig {
        namespace: namespace("banners", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::TOP | Anchor::RIGHT | Anchor::BOTTOM,
        size: LayerSize {
            width: LayerExtent::Px(u32::from(n.banner_width_px.0) + 2 * ROOM),
            height: LayerExtent::Fill,
        },
        exclusive: ExclusiveZone::Auto,
        margin: margin(n),
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::{ROOM, layer};
    use ds_settings::Px;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, Margin, RegionSpec,
    };
    use sill_settings::{NotificationsSettings, ShellFile};
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_column_hangs_down_the_right_edge_and_takes_nothing_until_a_card_shows() {
        let n = NotificationsSettings::default();
        let config = layer(
            &output(Some("DP-1")),
            &ShellFile {
                notifications: n.clone(),
                ..ShellFile::default()
            },
        );
        assert_eq!(config.namespace, "sill-banners-DP-1");
        assert_eq!(config.layer, LayerLevel::Overlay);
        assert_eq!(config.anchor, Anchor::TOP | Anchor::RIGHT | Anchor::BOTTOM);
        assert_eq!(config.size.width, LayerExtent::Px(320 + 2 * ROOM));
        assert_eq!(config.size.height, LayerExtent::Fill);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.input, RegionSpec::Empty);
        assert_eq!(config.blur, BlurSpec::None);
        assert_eq!(
            config.margin,
            Margin {
                top: -32,
                right: -32,
                bottom: -32,
                left: 0
            }
        );
        let wide = NotificationsSettings {
            banner_width_px: Px(420),
            position_right_px: Px(40),
            ..n
        };
        let config = layer(
            &output(Some("DP-1")),
            &ShellFile {
                notifications: wide,
                ..ShellFile::default()
            },
        );
        assert_eq!(config.size.width, LayerExtent::Px(420 + 2 * ROOM));
        assert_eq!(config.margin.right, 0);
    }
}
