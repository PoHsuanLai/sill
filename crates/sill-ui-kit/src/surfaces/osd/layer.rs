//! The indicator's layer surface (design/20 §1.7): Overlay, at the top right under the bar (as
//! current macOS) or centred above the dock, per `osd.position`; no keyboard, no input, blurred
//! behind the card (`Element("osd")`, quire's `Osd` root's `id`).
//!
//! Its exclusive zone is `Auto` (wire 0): it reserves nothing, and the compositor keeps it clear
//! of what others reserve, so at the top it sits below the bar's reserve (and at the bottom above
//! the dock's) without reading either's geometry. `osd.margin_px` is the gap from that reserve
//! to the card, and at the top right also from the output's right edge. The surface is the card plus [`ROOM`] on every side for the
//! Osd material's ambient shadow, so its margin is the gap less that room (negative when the
//! gap is smaller: the transparent room may overlap the reserve).

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::{OsdPosition, OsdSettings};

use crate::surfaces::naming::namespace;

/// The card's size: quire's `Osd` fixes the width at 296 (a control-center module spanning both
/// columns); the height is the title line over the Capsule level, at the card's own padding and
/// gap (design/20 §1.7 leaves it open; 01-LAYOUT "OSD: position, size: not specified").
pub const CARD_WIDTH: u32 = 296;
pub const CARD_HEIGHT: u32 = 80;

/// The card's element id: quire's `Osd { id: .. }`, and the blur region.
pub const CARD_ID: &str = "osd";

/// The Osd material's corner (`--r-tile`, design/03 §17: 12), which the card's `border-radius`
/// and the blur region both follow.
pub const CARD_RADIUS: i32 = 12;

/// Transparent room around the card for the material's ambient shadow (`0 12px 40px -12px`
/// reaches 28 px beside the card and 40 below it).
pub const ROOM: u32 = 40;

/// The anchor for `position`.
pub fn anchor(position: OsdPosition) -> Anchor {
    match position {
        OsdPosition::TopRight => Anchor::TOP | Anchor::RIGHT,
        OsdPosition::BottomCentre => Anchor::BOTTOM,
    }
}

/// The margin that puts the card `osd.margin_px` from the edge's reserve (and, at the top right,
/// from the right edge).
pub fn margin(osd: &OsdSettings) -> Margin {
    let gap = i32::from(osd.margin_px.0) - i32::try_from(ROOM).unwrap_or(i32::MAX);
    match osd.position {
        OsdPosition::BottomCentre => Margin {
            bottom: gap,
            ..Margin::ZERO
        },
        OsdPosition::TopRight => Margin {
            top: gap,
            right: gap,
            ..Margin::ZERO
        },
    }
}

/// The indicator's layer configuration on `output`.
pub fn osd_layer(output: &OutputInfo, osd: &OsdSettings) -> LayerConfig {
    LayerConfig {
        namespace: namespace("osd", output),
        layer: LayerLevel::Overlay,
        anchor: anchor(osd.position),
        size: LayerSize {
            width: LayerExtent::Px(CARD_WIDTH + 2 * ROOM),
            height: LayerExtent::Px(CARD_HEIGHT + 2 * ROOM),
        },
        exclusive: ExclusiveZone::Auto,
        margin: margin(osd),
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(RegionSpec::Element {
            id: ElementId::new(CARD_ID),
            radius: CARD_RADIUS,
            inflate: 0,
        }),
        input: RegionSpec::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::{CARD_HEIGHT, CARD_WIDTH, ROOM, osd_layer};
    use crate::surfaces::naming::tests::output;
    use ds_settings::Px;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, Margin, RegionSpec,
    };
    use sill_settings::{OsdPosition, OsdSettings};

    #[test]
    fn the_indicator_floats_over_its_edge_and_takes_nothing() {
        let config = osd_layer(&output(Some("DP-1")), &OsdSettings::default());
        assert_eq!(config.namespace, "sill-osd-DP-1");
        assert_eq!(config.layer, LayerLevel::Overlay);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.input, RegionSpec::Empty);
        assert!(matches!(
            config.blur,
            BlurSpec::Region(RegionSpec::Element { .. })
        ));
        assert_eq!(config.size.width, LayerExtent::Px(CARD_WIDTH + 2 * ROOM));
        assert_eq!(config.size.height, LayerExtent::Px(CARD_HEIGHT + 2 * ROOM));
    }

    #[test]
    fn position_and_gap_come_from_the_settings() {
        let with = |position, gap| OsdSettings {
            position,
            margin_px: Px(gap),
            ..OsdSettings::default()
        };
        let cases = [
            (
                with(OsdPosition::BottomCentre, 24),
                Anchor::BOTTOM,
                Margin {
                    bottom: -16,
                    ..Margin::ZERO
                },
            ),
            (
                with(OsdPosition::BottomCentre, 120),
                Anchor::BOTTOM,
                Margin {
                    bottom: 80,
                    ..Margin::ZERO
                },
            ),
            (
                with(OsdPosition::TopRight, 24),
                Anchor::TOP | Anchor::RIGHT,
                Margin {
                    top: -16,
                    right: -16,
                    ..Margin::ZERO
                },
            ),
            (
                with(OsdPosition::TopRight, 40),
                Anchor::TOP | Anchor::RIGHT,
                Margin::ZERO,
            ),
        ];
        for (osd, anchor, margin) in cases {
            let config = osd_layer(&output(Some("DP-1")), &osd);
            assert_eq!(config.anchor, anchor, "{osd:?}");
            assert_eq!(config.margin, margin, "{osd:?}");
        }
    }
}
