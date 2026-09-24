//! The wallpaper's layer surface: Background, every edge, ignoring exclusive zones, no input.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig, LayerSize, Margin,
    OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The wallpaper's layer configuration on `output`.
pub fn wallpaper_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("wallpaper", output),
        layer: Layer::Background,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Fill,
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::wallpaper_layer;
    use crate::surfaces::naming::tests::output;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, Layer, RegionSpec};

    #[test]
    fn the_wallpaper_fills_the_background_and_takes_no_input() {
        let config = wallpaper_layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "sill-wallpaper-DP-1");
        assert_eq!(config.layer, Layer::Background);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.exclusive, ExclusiveZone::Ignore);
        assert_eq!(config.input, RegionSpec::Empty);
        assert_eq!(config.blur, BlurSpec::None);
    }
}
