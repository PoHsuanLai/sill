//! The accounts sheet's layer surface: Overlay over the whole output, no exclusive zone, keyboard
//! `Exclusive` (a question that asks must take the keys), input everywhere so a click outside the
//! window reaches nothing behind it. The confirmation sheet's shape, with no blur: a window's
//! material has none.

use shell_host::{
    Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    Margin, OutputInfo, OutputTarget, RegionSpec,
};

use sill_settings::ShellFile;

use sill_ui_kit::surface::namespace;

/// The accounts sheet on `output`.
pub fn layer(output: &OutputInfo, _settings: &ShellFile) -> LayerConfig {
    LayerConfig {
        namespace: namespace("accounts-sheet", output),
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
    use super::layer as accounts_layer;
    use shell_host::{Anchor, BlurSpec, ExclusiveZone, KeyboardMode, LayerLevel, RegionSpec};
    use sill_settings::ShellFile;
    use sill_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_sheet_is_a_keyboard_exclusive_full_output_overlay_without_blur() {
        let layer = accounts_layer(&output(Some("DP-1")), &ShellFile::default());
        assert_eq!(layer.namespace, "sill-accounts-sheet-DP-1");
        assert_eq!(layer.layer, LayerLevel::Overlay);
        assert_eq!(layer.anchor, Anchor::ALL);
        assert_eq!(layer.exclusive, ExclusiveZone::Ignore);
        assert_eq!(layer.keyboard, KeyboardMode::Exclusive);
        assert_eq!(layer.input, RegionSpec::Whole);
        assert_eq!(layer.blur, BlurSpec::None);
    }
}
