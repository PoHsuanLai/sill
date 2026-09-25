//! The desktop widget layer's surface: the `Bottom` layer (design/20 §1.14: above the wallpaper,
//! below windows), the whole output, exclusive zone `Auto` (clear of the bar's and the dock's
//! reserve), no keyboard, input and blur empty until the surface sets them to its widgets'
//! frames (`RegionSpec::Elements` of each `widget_element_id`).
//!
//! Why `Bottom` and not a second `Background` surface above sill's wallpaper: the layer-shell
//! protocol does not order two surfaces on one layer, so a widget layer on `Background` may be
//! drawn under the wallpaper; `Bottom` is always above `Background` and below windows. With
//! `spaces.wallpaper_drawer = "cosmic"` the wallpaper is cosmic-bg's, on `Background` too.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, LayerConfig, LayerSize, Margin,
    OutputInfo, OutputTarget, RegionSpec,
};

use crate::surfaces::naming::namespace;

/// The desktop widgets on `output`.
pub fn desktop_layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        namespace: namespace("desktop-widgets", output),
        layer: Layer::Bottom,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: Extent::Fill,
            height: Extent::Fill,
        },
        exclusive: ExclusiveZone::Auto,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::None,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::desktop_layer;
    use crate::surfaces::naming::tests::output;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, Extent, KeyboardMode, Layer, OutputTarget, RegionSpec,
    };

    #[test]
    fn the_layer_is_under_windows_over_the_whole_output_and_takes_nothing_yet() {
        let info = output(Some("DP-1"));
        let config = desktop_layer(&info);
        assert_eq!(config.namespace, "sill-desktop-widgets-DP-1");
        assert_eq!(config.layer, Layer::Bottom);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.size.width, Extent::Fill);
        assert_eq!(config.size.height, Extent::Fill);
        assert_eq!(config.exclusive, ExclusiveZone::Auto);
        assert_eq!(config.keyboard, KeyboardMode::None);
        assert_eq!(config.output, OutputTarget::Output(info.id));
        assert_eq!(config.blur, BlurSpec::None);
        assert_eq!(config.input, RegionSpec::Empty);
    }
}
