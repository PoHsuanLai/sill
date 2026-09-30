//! The bar's layer surface (design/20 §1.1): Top, anchored top, left and right, the settings
//! height, reserving it, no keyboard, blurred and hit-tested whole.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};
use sill_settings::ShellFile;

use crate::surface::namespace;

/// The bar's layer configuration on `output`.
pub(super) fn layer(output: &OutputInfo, settings: &ShellFile) -> LayerConfig {
    let height = u32::from(settings.bar.height_px.0);
    LayerConfig {
        namespace: namespace("bar", output),
        layer: LayerLevel::Top,
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Px(height),
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
    use super::layer;
    use crate::harness::fake::output_named as output;
    use ds_settings::Px;
    use shell_host::{
        Anchor, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, OutputId, OutputTarget,
        RegionSpec,
    };
    use sill_settings::{BarSettings, ShellFile};

    #[test]
    fn the_bar_reserves_its_settings_height() {
        for height in [32, 24] {
            let settings = ShellFile {
                bar: BarSettings {
                    height_px: Px(height),
                    ..BarSettings::default()
                },
                ..ShellFile::default()
            };
            let config = layer(&output(Some("DP-1")), &settings);
            assert_eq!(config.namespace, "sill-bar-DP-1");
            assert_eq!(config.layer, LayerLevel::Top);
            assert_eq!(config.anchor, Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
            assert_eq!(config.size.width, LayerExtent::Fill);
            assert_eq!(config.size.height, LayerExtent::Px(u32::from(height)));
            assert_eq!(config.exclusive, ExclusiveZone::Reserve(u32::from(height)));
            assert_eq!(config.keyboard, KeyboardMode::None);
            assert_eq!(config.output, OutputTarget::Output(OutputId(7)));
            assert_eq!(config.input, RegionSpec::Whole);
        }
    }
}
