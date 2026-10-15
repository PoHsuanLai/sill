//! The picker's layer surface: Overlay over the whole output, no exclusive zone, keyboard
//! `Exclusive` (Escape and the arrows must reach it), input everywhere (a press outside the tiles
//! turns the choice down), no blur: the backdrop only dims.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_settings::ShellFile;

use sill_ui_kit::surface::namespace;

/// The picker on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("split-picker", output),
        layer: LayerLevel::Overlay,
        anchor: Anchor::ALL,
        size: LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
        },
        exclusive: ExclusiveZone::Ignore,
        margin: Margin::ZERO,
        keyboard: KeyboardMode::Exclusive,
        output: OutputTarget::Output(output.id),
        blur: BlurSpec::None,
        input: RegionSpec::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::layer as picker_layer;
    use shell_host::{
        Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, RegionSpec,
    };
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_picker_is_a_keyboard_exclusive_full_output_overlay_taking_input_everywhere() {
        let layer = picker_layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(layer.namespace, "sill-split-picker-DP-1");
        assert_eq!(layer.layer, LayerLevel::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(
            (layer.size.width, layer.size.height),
            (LayerExtent::Fill, LayerExtent::Fill)
        );
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
        assert_eq!(layer.blur, BlurSpec::None);
    }
}
