//! The bar's layer surface (design/20 §1.1): Top, anchored top, left and right, the settings
//! height, reserving it, no keyboard, blurred and hit-tested whole.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig, LayerSize, Margin,
    OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::BarSettings;

use crate::surfaces::naming::namespace;

/// The bar's layer configuration on `output`.
pub fn bar_layer(output: &OutputInfo, bar: &BarSettings) -> LayerConfig {
    let height = u32::from(bar.height_px.0);
    LayerConfig {
        namespace: namespace("bar", output),
        layer: Layer::Top,
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Px(height),
        },
        exclusive: ExclusiveZone::Reserve(height),
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::Region(RegionSpec::Whole),
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::bar_layer;
    use crate::surfaces::naming::tests::output;
    use ds_settings::Px;
    use shell_host::{
        Anchor, ExclusiveZone, Extent, KeyboardMode, Layer, OutputId, OutputTarget, RegionSpec,
    };
    use sill_settings::BarSettings;

    #[test]
    fn the_bar_reserves_its_settings_height() {
        for height in [32, 24] {
            let bar = BarSettings {
                height_px: Px(height),
                ..BarSettings::default()
            };
            let config = bar_layer(&output(Some("DP-1")), &bar);
            assert_eq!(config.namespace, "sill-bar-DP-1");
            assert_eq!(config.layer, Layer::Top);
            assert_eq!(config.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
            assert_eq!(config.size.width, Extent::Fill);
            assert_eq!(config.size.height, Extent::Px(u32::from(height)));
            assert_eq!(config.exclusive, ExclusiveZone::Reserve(u32::from(height)));
            assert_eq!(config.keyboard, KeyboardMode::None);
            assert_eq!(config.output, OutputTarget::Output(OutputId(7)));
            assert_eq!(config.input, RegionSpec::Whole);
        }
    }
}
