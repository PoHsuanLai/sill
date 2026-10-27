//! A hot corner's surface: Overlay, anchored to its two edges, a 2 px square, exclusive zone
//! `Ignore` (at the true corner of the output, over the bar's and the dock's reserve), no
//! keyboard, no blur, input the whole square.

use ds::base::word::Word;
use ds_behaviour::hot_corner::CornerParams;
use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_ui_kit::links::Corner;
use sill_ui_kit::surface::namespace;

/// `corner` of `output`, [`CornerParams::HOT_SPOT_PX`] logical px square.
pub fn layer(output: &OutputInfo, corner: Corner) -> LayerConfig {
    let anchor = match corner {
        Corner::TopLeft => Anchor::TOP | Anchor::LEFT,
        Corner::TopRight => Anchor::TOP | Anchor::RIGHT,
        Corner::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
        Corner::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
    };
    let side = u32::from(CornerParams::HOT_SPOT_PX);
    LayerConfig {
        namespace: namespace(&format!("hot-corner-{}", corner.slug()), output),
        layer: LayerLevel::Overlay,
        anchor,
        size: LayerSize {
            width: LayerExtent::Px(side),
            height: LayerExtent::Px(side),
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::layer;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, RegionSpec,
    };
    use sill_ui_kit::harness::fake::output_named as output;
    use sill_ui_kit::links::Corner;

    #[test]
    fn each_corner_is_a_tiny_square_at_its_corner_over_everything() {
        let info = output(Some("DP-1"));
        let cases = [
            (
                Corner::TopLeft,
                Anchor::TOP | Anchor::LEFT,
                "sill-hot-corner-top-left-DP-1",
            ),
            (
                Corner::TopRight,
                Anchor::TOP | Anchor::RIGHT,
                "sill-hot-corner-top-right-DP-1",
            ),
            (
                Corner::BottomLeft,
                Anchor::BOTTOM | Anchor::LEFT,
                "sill-hot-corner-bottom-left-DP-1",
            ),
            (
                Corner::BottomRight,
                Anchor::BOTTOM | Anchor::RIGHT,
                "sill-hot-corner-bottom-right-DP-1",
            ),
        ];
        for (corner, anchor, name) in cases {
            let config = layer(&info, corner);
            assert_eq!(config.namespace, name);
            assert_eq!(config.anchor, anchor, "{corner:?}");
            assert_eq!(config.layer, LayerLevel::Overlay);
            assert_eq!(config.size.width, LayerExtent::Px(2));
            assert_eq!(config.size.height, LayerExtent::Px(2));
            assert_eq!(config.exclusive, ExclusiveZone::Ignore);
            assert_eq!(config.keyboard, KeyboardMode::None);
            assert_eq!(config.blur, BlurSpec::None);
            assert_eq!(config.input, RegionSpec::Whole);
        }
    }
}
