//! The indicator's layer surface (design/20 §1.7): Overlay, at the top right under the bar (as
//! current macOS); no keyboard, no input, blurred
//! behind the card (`Element("osd")`, quire's `Osd` root's `id`).
//!
//! Its exclusive zone is `Auto` (wire 0): it reserves nothing, and the compositor keeps it clear
//! of what others reserve, so it sits below the bar's reserve without reading its geometry.
//! `--osd-margin` is the gap from that reserve to the card, and also from the output's right
//! edge. The surface is the card plus [`ROOM`] on every side for the
//! Osd material's ambient shadow, so its margin is the gap less that room (negative when the
//! gap is smaller: the transparent room may overlap the reserve).

use shell_host::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel,
    LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::ShellFile;

use super::tokens::OsdTokens;
use sill_ui_kit::surface::namespace;

/// The card's size: quire's `Osd` fixes the width at 296 (a control-center module spanning both
/// columns); the height is the title line over the level, at the card's own padding and
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

/// The corner the indicator sits in: top right.
pub fn anchor() -> Anchor {
    Anchor::TOP | Anchor::RIGHT
}

/// The margin that puts the card `--osd-margin` from the bar's reserve and from the right edge.
pub fn margin(tokens: &OsdTokens) -> Margin {
    let gap = i32::from(tokens.margin) - i32::try_from(ROOM).unwrap_or(i32::MAX);
    Margin {
        top: gap,
        right: gap,
        ..Margin::ZERO
    }
}

/// The indicator's layer configuration on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("osd", output),
        layer: LayerLevel::Overlay,
        anchor: anchor(),
        size: LayerSize {
            width: LayerExtent::Px(CARD_WIDTH + 2 * ROOM),
            height: LayerExtent::Px(CARD_HEIGHT + 2 * ROOM),
        },
        exclusive: ExclusiveZone::Auto,
        margin: margin(&OsdTokens::default()),
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
    use super::{CARD_HEIGHT, CARD_WIDTH, ROOM, anchor, layer, margin};
    use crate::osd::tokens::OsdTokens;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, Margin, RegionSpec,
    };
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_indicator_floats_over_its_edge_and_takes_nothing() {
        let config = layer(&output(Some("DP-1")), &ShellFile::default());
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
    fn the_gap_comes_from_the_token() {
        let with = |margin| OsdTokens { margin };
        let cases = [
            (
                with(24),
                Margin {
                    top: -16,
                    right: -16,
                    ..Margin::ZERO
                },
            ),
            (with(40), Margin::ZERO),
        ];
        for (tokens, want) in cases {
            assert_eq!(margin(&tokens), want, "{tokens:?}");
        }
        assert_eq!(margin(&OsdTokens::default()).top, -16);
        assert_eq!(anchor(), Anchor::TOP | Anchor::RIGHT);
    }
}
