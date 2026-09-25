//! The indicator's layer surface (design/20 §1.7): Overlay, anchored to the `osd.position` edge
//! and centred along it, no keyboard, no input, blurred behind the card (`Element("osd")`).
//!
//! Its exclusive zone is `Auto` (wire 0): it reserves nothing, and the compositor keeps it clear
//! of what others reserve, so at the bottom it sits above the dock's reserve (and at the top
//! below the bar's) without reading the dock's geometry. `osd.bottom_margin_px` is the gap
//! from that reserve to the card. The surface is the card plus [`ROOM`] on every side for the
//! Osd material's ambient shadow, so its margin is the gap less that room (negative when the
//! gap is smaller: the transparent room may overlap the reserve).

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::{OsdPosition, OsdSettings};

use crate::surfaces::naming::namespace;

/// The card's size (design/20 §1.7 leaves it open; 01-LAYOUT "OSD: position, size: not
/// specified"): room for the 22 px glyph and a level bar a macOS indicator's width.
pub const CARD_WIDTH: u32 = 280;
pub const CARD_HEIGHT: u32 = 52;

/// The card's element id: the blur region.
pub const CARD_ID: &str = "osd";

/// The Osd material's corner (design/03 §17: 18), which the blur region follows.
pub const CARD_RADIUS: i32 = 18;

/// Transparent room around the card for the material's ambient shadow (`0 12px 40px -12px`
/// reaches 28 px beside the card and 40 below it).
pub const ROOM: u32 = 40;

/// The anchor for `position`.
pub fn anchor(position: OsdPosition) -> Anchor {
    match position {
        OsdPosition::Bottom => Anchor::BOTTOM,
        OsdPosition::Top => Anchor::TOP,
    }
}

/// The margin that puts the card `osd.bottom_margin_px` from the edge's reserve.
pub fn margin(osd: &OsdSettings) -> Margin {
    let gap = i32::from(osd.bottom_margin_px.0) - i32::try_from(ROOM).unwrap_or(i32::MAX);
    match osd.position {
        OsdPosition::Bottom => Margin {
            bottom: gap,
            ..Margin::ZERO
        },
        OsdPosition::Top => Margin {
            top: gap,
            ..Margin::ZERO
        },
    }
}

/// The indicator's layer configuration on `output`.
pub fn osd_layer(output: &OutputInfo, osd: &OsdSettings) -> LayerConfig {
    LayerConfig {
        namespace: namespace("osd", output),
        layer: Layer::Overlay,
        anchor: anchor(osd.position),
        size: LayerSize {
            width: Extent::Px(CARD_WIDTH + 2 * ROOM),
            height: Extent::Px(CARD_HEIGHT + 2 * ROOM),
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
        Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, Margin, RegionSpec,
    };
    use sill_settings::{OsdPosition, OsdSettings};

    #[test]
    fn the_indicator_floats_over_its_edge_and_takes_nothing() {
        let config = osd_layer(&output(Some("DP-1")), &OsdSettings::default());
        assert_eq!(config.namespace, "sill-osd-DP-1");
        assert_eq!(config.layer, Layer::Overlay);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.input, RegionSpec::Empty);
        assert!(matches!(
            config.blur,
            BlurSpec::Region(RegionSpec::Element { .. })
        ));
        assert_eq!(config.size.width, Extent::Px(CARD_WIDTH + 2 * ROOM));
        assert_eq!(config.size.height, Extent::Px(CARD_HEIGHT + 2 * ROOM));
    }

    #[test]
    fn position_and_gap_come_from_the_settings() {
        let with = |position, gap| OsdSettings {
            position,
            bottom_margin_px: Px(gap),
            ..OsdSettings::default()
        };
        let cases = [
            (
                with(OsdPosition::Bottom, 24),
                Anchor::BOTTOM,
                Margin {
                    bottom: -16,
                    ..Margin::ZERO
                },
            ),
            (
                with(OsdPosition::Bottom, 120),
                Anchor::BOTTOM,
                Margin {
                    bottom: 80,
                    ..Margin::ZERO
                },
            ),
            (with(OsdPosition::Top, 40), Anchor::TOP, Margin::ZERO),
        ];
        for (osd, anchor, margin) in cases {
            let config = osd_layer(&output(Some("DP-1")), &osd);
            assert_eq!(config.anchor, anchor, "{osd:?}");
            assert_eq!(config.margin, margin, "{osd:?}");
        }
    }
}
