//! The wallpaper's layer surface: Background, every edge, ignoring exclusive zones, no keyboard,
//! input over the whole output (a press of the secondary button opens the desktop's menu).

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_ui_kit::surface::namespace;

/// The wallpaper's layer configuration on `output`.
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("wallpaper", output),
        layer: LayerLevel::Background,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
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
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, LayerLevel, RegionSpec};
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_wallpaper_fills_the_background_and_takes_the_pointer_for_its_menu() {
        let config = layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "sill-wallpaper-DP-1");
        assert_eq!(config.layer, LayerLevel::Background);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.exclusive, ExclusiveZone::Ignore);
        assert_eq!(config.input, RegionSpec::Whole);
        assert_eq!(config.blur, BlurSpec::None);
    }
}
